# The build host

`vm.py` drives the Windows XP build host of DeForm6 from the Mac. The host is
a UTM virtual machine with the Visual Basic 6 IDE. It is offline. QEMU gives
a QMP socket on `127.0.0.1:4444` and a serial port on a pty.

The script uses only the standard library of Python 3.

## Before a run

- A `cmd` window must be open in the machine, with the keyboard focus.
- Do not log in on the Welcome screen, and do not type a password. When the
  machine shows the Welcome screen, the user must log in.
- Do not start `LockWorkStation.exe` in the machine: it locks the session.
  `runs.bat` does not start it.
- When the firewall asks about a program, the user answers. Escape closes the
  question and changes no setting.

## A build and a start check

1. Export the lifted corpus and make a CD image of it:

   ```
   cargo run -p xtask -- export-lift-builds <dir>
   hdiutil makehybrid -iso -joliet -default-volume-name DEFORM6 -o <dir>.iso <dir>
   ```

2. Put the image into the CD drive:

   ```
   python3 scripts/vm/vm.py cd <dir>.iso
   ```

   Autoplay opens a dialog for the new CD, which takes the keyboard. Wait
   30 seconds, take a picture, and press Escape.

3. Start the reader of the serial port and the watcher of the markers:

   ```
   python3 scripts/vm/vm.py read <capture> &
   python3 scripts/vm/vm.py shots <capture> <pictures> 2 &
   ```

4. Type the commands into the machine from a file, one command on each line:

   ```
   xcopy D:\*.* E:\deform6\<run>\ /e /i /q
   attrib -r E:\deform6\<run>\*.* /s
   cd /d E:\deform6\<run>
   build.bat "E:\Program Files\Microsoft Visual Studio\VB98\VB6.EXE" & sendlogs.bat & runs.bat
   ```

   ```
   python3 scripts/vm/vm.py type <file>
   ```

   `build.bat` builds each original and each lifted project. `sendlogs.bat`
   sends the logs through the serial port, and then `===END===`. `runs.bat`
   starts each program, sends `===SHOT pNN side===` while its first window
   shows, and sends `===END===` at the end. The watcher takes a picture at
   each marker, and stops at the second `===END===`.

5. Read the results:

   ```
   python3 scripts/vm/vm.py builds <capture>
   python3 scripts/vm/vm.py errors <capture> <dir>/extracted
   python3 scripts/vm/vm.py compare <pictures>
   ```

   `compare` gives, for each program, the share of the pixels above the task
   bar that differ between the original and the rebuilt program. A share of
   0 is the same screen. A share above 0 is a difference to look at: a form
   with no recovered properties opens at another place and size.

## What each command does

| Command | Result |
|---|---|
| `status` | The state of the machine |
| `shot OUT.png` | A picture of the screen |
| `keys KEY...` | Presses each key or combination, such as `esc` or `ctrl-c` |
| `type FILE` | Types each line of the file, then Return |
| `cd ISO` | Copies the image into the sandbox of QEMU and puts it into the CD drive |
| `nudge` | Moves the mouse one pixel and back, which stops the screen saver |
| `read OUT` | Copies the serial port into the file, until the process ends |
| `shots CAPTURE DIR [ENDS]` | Takes a picture at each marker, until `ENDS` end markers |
| `compare DIR` | Compares the pictures of each program |
| `builds CAPTURE` | Counts the builds that succeeded on each side |
| `errors CAPTURE EXTRACTED` | Gives the first compile error of each lifted project, with its line |

## Why these details

- Each file that QEMU reads or writes must be in
  `~/Library/Containers/com.utmapp.QEMUHelper/Data/tmp`, because QEMU runs in
  the sandbox of UTM.
- A key press holds for 40 ms, with 0.12 s between keys. Faster typing
  overlapped the keys.
- `read` keeps the pty open while it sets the raw mode. When the pty closes
  between the two, it turns CR into LF again.
- VB6 gives the line of an error one line after the count from the last
  `Attribute` line.
- QEMU serves one QMP client at a time. A second client waits until the
  first closes its connection. Thus `shots` connects only for each picture
  and each move of the mouse, and `type` can connect while `shots` runs.
