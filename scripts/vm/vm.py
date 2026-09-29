#!/usr/bin/env python3
"""Drives the Windows XP build host of DeForm6 from the Mac.

The host is a UTM virtual machine with Visual Basic 6. QEMU gives a QMP
socket on 127.0.0.1:4444 and a serial port on a pty. See README.md in this
folder for the order of the steps.

    vm.py status                      prints the state of the machine
    vm.py shot OUT.png                writes a picture of the screen
    vm.py keys KEY...                 presses keys, such as esc or ctrl-c
    vm.py type FILE                   types each line of FILE, then Return
    vm.py cd ISO                      puts ISO into the CD drive
    vm.py nudge                       moves the mouse by one pixel and back
    vm.py read OUT                    copies the serial port into OUT
    vm.py shots CAPTURE DIR [ENDS]    takes a picture at each run marker
    vm.py compare DIR                 compares the pictures of each program
    vm.py builds CAPTURE              counts the builds that succeeded
    vm.py errors CAPTURE EXTRACTED    gives the first error of each project
"""

import json
import os
import re
import shutil
import socket
import struct
import subprocess
import sys
import termios
import time
import tty
import zlib

QMP_ADDRESS = ("127.0.0.1", 4444)

# QEMU runs in the sandbox of UTM, so each file that QEMU reads or writes
# must be in this folder.
SANDBOX = os.path.expanduser("~/Library/Containers/com.utmapp.QEMUHelper/Data/tmp")

# The QOM path of the CD drive of the machine.
CD_DEVICE = "/machine/peripheral-anon/device[5]"

# The label of the character device of the serial port.
SERIAL_LABEL = "term0"

# The rows at the bottom of the screen that the task bar and its clock take.
TASKBAR_ROWS = 30

SHIFTED = {
    "!": "1", "@": "2", "#": "3", "$": "4", "%": "5", "^": "6", "&": "7",
    "*": "8", "(": "9", ")": "0", "_": "minus", "+": "equal",
    "{": "bracket_left", "}": "bracket_right", "|": "backslash",
    ":": "semicolon", '"': "apostrophe", "<": "comma", ">": "dot",
    "?": "slash", "~": "grave_accent",
}
PLAIN = {
    " ": "spc", "-": "minus", "=": "equal", "[": "bracket_left",
    "]": "bracket_right", "\\": "backslash", ";": "semicolon",
    "'": "apostrophe", ",": "comma", ".": "dot", "/": "slash",
    "`": "grave_accent", "\n": "ret",
}


class Qmp:
    """One connection to the QMP socket."""

    def __init__(self):
        self.socket = socket.create_connection(QMP_ADDRESS)
        self.file = self.socket.makefile("rwb")
        self.read()
        self.command("qmp_capabilities")

    def close(self):
        self.file.close()
        self.socket.close()

    def read(self):
        while True:
            message = json.loads(self.file.readline())
            if "event" not in message:
                return message

    def command(self, name, **arguments):
        self.file.write(json.dumps({"execute": name, "arguments": arguments}).encode() + b"\n")
        self.file.flush()
        reply = self.read()
        if "error" in reply:
            raise SystemExit(reply)
        return reply.get("return")

    def combo(self, keys):
        # A hold of 40 ms and 0.12 s between keys: faster typing overlapped
        # keys and fired a shortcut of the host.
        self.command(
            "send-key",
            keys=[{"type": "qcode", "data": key} for key in keys],
            **{"hold-time": 40},
        )
        time.sleep(0.12)

    def character(self, char):
        if char.isalpha():
            self.combo(["shift", char.lower()] if char.isupper() else [char])
        elif char.isdigit():
            self.combo([char])
        elif char in SHIFTED:
            self.combo(["shift", SHIFTED[char]])
        elif char in PLAIN:
            self.combo([PLAIN[char]])
        else:
            raise SystemExit(f"no key for {char!r}")


def ppm_to_png(ppm, out):
    data = open(ppm, "rb").read()
    parts = data.split(b"\n", 3)
    width, height = map(int, parts[1].split())
    pixels = parts[3]
    raw = b"".join(
        b"\0" + pixels[y * width * 3:(y + 1) * width * 3] for y in range(height)
    )

    def chunk(kind, body):
        return (
            struct.pack(">I", len(body)) + kind + body
            + struct.pack(">I", zlib.crc32(kind + body))
        )

    header = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    open(out, "wb").write(
        b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b"")
    )


def load_png(path):
    """Reads a picture that shot writes: 8-bit RGB rows with filter 0."""
    data = open(path, "rb").read()
    width, height = struct.unpack(">II", data[16:24])
    at, idat = 8, b""
    while at < len(data):
        length = struct.unpack(">I", data[at:at + 4])[0]
        if data[at + 4:at + 8] == b"IDAT":
            idat += data[at + 8:at + 8 + length]
        at += 12 + length
    raw = zlib.decompress(idat)
    stride = width * 3 + 1
    rows = [raw[y * stride + 1:(y + 1) * stride] for y in range(height)]
    return width, height, rows


def shot(qmp, out):
    ppm = os.path.join(SANDBOX, "qmp-shot.ppm")
    if os.path.exists(ppm):
        os.remove(ppm)
    qmp.command("screendump", filename=ppm)
    for _ in range(50):
        if os.path.exists(ppm) and os.path.getsize(ppm) > 0:
            break
        time.sleep(0.1)
    time.sleep(0.3)
    ppm_to_png(ppm, out)


def nudge(qmp):
    for value in (1, -1):
        qmp.command(
            "input-send-event",
            events=[{"type": "rel", "data": {"axis": "x", "value": value}}],
        )


def serial_path(qmp):
    for device in qmp.command("query-chardev"):
        if device.get("label") == SERIAL_LABEL:
            return device.get("filename", "").removeprefix("pty:")
    raise SystemExit("the machine has no serial pty")


def read_serial(out):
    """Copies the serial port into OUT until the process ends.

    The pty stays open while the mode changes to raw: when it closes
    between the two, the pty turns CR into LF again.
    """
    path = serial_path(Qmp())
    with open(path, "rb", buffering=0) as port, open(out, "wb", buffering=0) as target:
        tty.setraw(port.fileno())
        mode = termios.tcgetattr(port.fileno())
        mode[4] = mode[5] = termios.B115200
        termios.tcsetattr(port.fileno(), termios.TCSANOW, mode)
        while True:
            data = port.read(4096)
            if data:
                target.write(data)


MARKER = re.compile(r"===(SHOT|NONE) (p\d\d) (\S+)===")


def watch_shots(capture, folder, ends):
    """Takes a picture at each ===SHOT pNN side=== marker of CAPTURE.

    Stops after ENDS ===END=== markers. Moves the mouse once a minute, so
    that the screen saver does not start during the run. QEMU serves one QMP
    client at a time, so the watcher connects only for each picture and each
    move, and `type` can connect between them.
    """
    os.makedirs(folder, exist_ok=True)
    seen, last_nudge = 0, 0.0
    while True:
        if time.time() - last_nudge > 60:
            qmp = Qmp()
            nudge(qmp)
            qmp.close()
            last_nudge = time.time()
        try:
            data = open(capture, "rb").read().decode("latin-1")
        except FileNotFoundError:
            data = ""
        marks = MARKER.findall(data)
        for kind, short, side in marks[seen:]:
            if kind == "SHOT":
                qmp = Qmp()
                shot(qmp, os.path.join(folder, f"{short}-{side}.png"))
                qmp.close()
            print(kind, short, side, flush=True)
        seen = len(marks)
        if data.count("===END===") >= ends:
            return
        time.sleep(0.3)


def compare(folder):
    """Prints the share of the pixels above the task bar that differ between
    the picture of the original and the rebuilt program."""
    shorts = sorted({
        match.group(1)
        for name in os.listdir(folder)
        if (match := re.match(r"(p\d\d)-", name))
    })
    same = 0
    for short in shorts:
        paths = [os.path.join(folder, f"{short}-{side}.png") for side in ("original", "extracted")]
        if not all(os.path.exists(path) for path in paths):
            print(short, "missing")
            continue
        width, height, first = load_png(paths[0])
        _, _, second = load_png(paths[1])
        differ = 0
        for y in range(height - TASKBAR_ROWS):
            if first[y] != second[y]:
                differ += sum(
                    first[y][x * 3:x * 3 + 3] != second[y][x * 3:x * 3 + 3]
                    for x in range(width)
                )
        same += differ == 0
        print(f"{short} {differ / (width * (height - TASKBAR_ROWS)):.4f} {differ}")
    print(f"{same} of {len(shorts)} pairs are the same")


def capture_files(capture):
    data = open(capture, "rb").read().decode("latin-1").replace("\r\n", "\n")
    return re.findall(r"===FILE \d+ ([^\n=]*)===\n(.*?)===EOF===", data, re.S)


def builds(capture):
    files = capture_files(capture)
    for side in ("original", "extracted"):
        logs = [body for path, body in files if path.endswith(f"-{side}.txt")]
        done = sum("succeeded" in body for body in logs)
        print(f"{side}: {done} of {len(logs)} builds succeeded")


def errors(capture, extracted):
    """Prints the first compile error of each lifted project with its line.

    VB6 counts the lines of the code after the last Attribute line, and its
    number is one line after that count.
    """
    for path, body in capture_files(capture):
        match = re.match(r"logs\\(p\d\d)-extracted\.txt", path.strip())
        if not match:
            continue
        text = re.sub(r"\n+", " ", body).strip()
        found = re.search(r"File '.*\\([^\\']+)', Line (\d+) : (.*?)(?: Build|$)", text)
        if not found:
            continue
        source = os.path.join(extracted, match.group(1), found.group(1))
        lines = open(source, encoding="latin-1").read().splitlines()
        attributes = [i for i, line in enumerate(lines) if line.startswith("Attribute VB_")]
        base = attributes[-1] + 1 if attributes else 0
        index = base + int(found.group(2))
        line = lines[index].strip() if index < len(lines) else "?"
        print(f"{match.group(1)} {found.group(1)}:{found.group(2)} {found.group(3).strip()[:40]} | {line[:150]}")


def main():
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    what, rest = sys.argv[1], sys.argv[2:]
    if what == "status":
        print(Qmp().command("query-status"))
    elif what == "shot":
        shot(Qmp(), rest[0])
    elif what == "keys":
        qmp = Qmp()
        for key in rest:
            qmp.combo(key.split("-"))
    elif what == "type":
        qmp = Qmp()
        for line in open(rest[0], encoding="ascii").read().splitlines():
            for char in line:
                qmp.character(char)
            qmp.character("\n")
    elif what == "cd":
        target = os.path.join(SANDBOX, os.path.basename(rest[0]))
        shutil.copyfile(rest[0], target)
        Qmp().command(
            "blockdev-change-medium", id=CD_DEVICE, filename=target,
            format="raw", **{"read-only-mode": "read-only"},
        )
    elif what == "nudge":
        nudge(Qmp())
    elif what == "read":
        read_serial(rest[0])
    elif what == "shots":
        watch_shots(rest[0], rest[1], int(rest[2]) if len(rest) > 2 else 1)
    elif what == "compare":
        compare(rest[0])
    elif what == "builds":
        builds(rest[0])
    elif what == "errors":
        errors(rest[0], rest[1])
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main()
