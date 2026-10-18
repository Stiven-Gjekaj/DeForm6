//! The steps that the behaviour check of `export-lift-builds` sends to each
//! program, past its first screen.
//!
//! `scripts/vm/steps.toml` holds a list of keys for each program of the
//! P-code corpus. `drive.vbs` starts the program on the Windows host, sends
//! `===SHOT <short> <side>===` through `COM1` while its first window shows,
//! and then sends each step to the program with `SendKeys`, with
//! `===SHOT <short> <side>-s<n>===` after step `n`. The other machine takes a
//! picture at each marker, and `vm.py compare` compares the picture of each
//! step of the original with the picture of the same step of the rebuilt
//! program.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::builds::crlf_text;

/// The file of the steps, relative to the root of the workspace.
const STEPS_FILE: &str = "scripts/vm/steps.toml";

/// The text of `scripts/vm/steps.toml`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StepsFile {
    program: Vec<ProgramSteps>,
}

/// The steps of one program.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProgramSteps {
    /// The path of the executable, relative to `corpus-pcode/`.
    key: String,
    /// The keys of each step, in the syntax of `SendKeys`.
    steps: Vec<String>,
    /// Windows puts the window of the program, at another place for each
    /// new window, so `vm.py compare` compares the window at its own place.
    /// The export does not use it.
    #[serde(default)]
    #[allow(dead_code, reason = "vm.py reads it; the export only checks the file")]
    window: bool,
    /// The boxes of pixels that a random value draws, which `vm.py compare`
    /// leaves out.
    #[serde(default)]
    random: Vec<RandomBox>,
}

/// A box of pixels that a random value draws on some screens of a program.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RandomBox {
    /// The screens of the box: "" is the first window and "s1" the first step.
    screens: Vec<String>,
    /// The left, top, right and bottom pixel of the box, all in the box.
    #[serde(rename = "box")]
    corners: [u32; 4],
    /// What draws the random value.
    #[allow(dead_code, reason = "a reader of steps.toml reads it; no code does")]
    why: String,
}

/// Gives the path of `scripts/vm/steps.toml`.
pub(crate) fn steps_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(STEPS_FILE)
}

/// Reads the steps of each program from `text`, by its key. A key that comes
/// twice, and a step that is empty or holds a character that is not
/// printable ASCII, are errors: `drive.vbs` reads one step from each line.
pub(crate) fn parse_steps(text: &str) -> Result<BTreeMap<String, Vec<String>>, String> {
    let file: StepsFile = toml::from_str(text).map_err(|err| format!("{STEPS_FILE}: {err}"))?;
    let mut out = BTreeMap::new();
    for program in file.program {
        for step in &program.steps {
            if step.is_empty() || !step.bytes().all(|byte| (b' '..=b'~').contains(&byte)) {
                return Err(format!(
                    "{STEPS_FILE}: {}: the step {step:?} is empty or holds a character that \
                     is not printable ASCII",
                    program.key
                ));
            }
        }
        for random in &program.random {
            let [left, top, right, bottom] = random.corners;
            let screens = random.screens.iter().all(|screen| {
                screen.is_empty()
                    || screen
                        .strip_prefix('s')
                        .and_then(|step| step.parse::<usize>().ok())
                        .is_some_and(|step| (1..=program.steps.len()).contains(&step))
            });
            if left > right || top > bottom || random.screens.is_empty() || !screens {
                return Err(format!(
                    "{STEPS_FILE}: {}: the random box {:?} for the screens {:?} is empty or \
                     names a screen that the program does not have",
                    program.key, random.corners, random.screens
                ));
            }
        }
        if out.insert(program.key.clone(), program.steps).is_some() {
            return Err(format!("{STEPS_FILE}: {} comes twice", program.key));
        }
    }
    Ok(out)
}

/// Gives the text of `steps\<short>.txt`: one step on each line, with CRLF.
pub(crate) fn render_steps(steps: &[String]) -> String {
    crlf_text(steps)
}

/// The script that starts one program and sends its steps. `runs.bat`
/// calls it with the path of the executable, the short name and the side.
/// It waits after each marker, so that the other machine takes its picture
/// before the next keys arrive. It sends the keys of a step only while the
/// program runs and `AppActivate` brings it to the front: keys that go to
/// the window of `runs.bat` can run a command there. A step whose keys it
/// does not send gives `===NOKEYS <short> <side>-s<n>===` before its marker.
/// After the keys of a step, it waits until the program uses no processor
/// time for 3 samples of half a second, for at most 20 seconds: a P-code
/// program can redraw for longer than the fixed wait, and the picture then
/// shows the middle of its work.
const DRIVE_LINES: &[&str] = &[
    "' Starts one program, sends a marker through COM1 while its first window",
    "' shows, and sends each step of steps\\<short>.txt to it with SendKeys,",
    "' with a marker after each step.",
    "Option Explicit",
    "Dim shell, files, program, short, side, path, list, keys, count, wmi",
    "Set shell = CreateObject(\"WScript.Shell\")",
    "Set files = CreateObject(\"Scripting.FileSystemObject\")",
    "Set wmi = GetObject(\"winmgmts:\\\\.\\root\\cimv2\")",
    "short = WScript.Arguments(1)",
    "side = WScript.Arguments(2)",
    "Set program = shell.Exec(\"\"\"\" & WScript.Arguments(0) & \"\"\"\")",
    "WScript.Sleep 6000",
    "Mark side",
    "path = files.GetAbsolutePathName(\"steps\\\" & short & \".txt\")",
    "If files.FileExists(path) Then",
    "  Set list = files.OpenTextFile(path, 1)",
    "  count = 0",
    "  Do Until list.AtEndOfStream",
    "    keys = list.ReadLine",
    "    count = count + 1",
    "    If program.Status = 0 And shell.AppActivate(program.ProcessID) Then",
    "      WScript.Sleep 500",
    "      shell.SendKeys keys",
    "    Else",
    "      shell.Run \"cmd /c >COM1 echo ===NOKEYS \" & short & \" \" & side & \"-s\" & count & \"===\", 0, True",
    "    End If",
    "    WScript.Sleep 2500",
    "    WaitIdle",
    "    Mark side & \"-s\" & count",
    "  Loop",
    "  list.Close",
    "End If",
    "If program.Status = 0 Then program.Terminate",
    "",
    "Sub WaitIdle()",
    "  Dim quiet, samples, busy, item",
    "  quiet = 0",
    "  samples = 0",
    "  Do While quiet < 3 And samples < 40 And program.Status = 0",
    "    WScript.Sleep 500",
    "    samples = samples + 1",
    "    busy = False",
    "    For Each item In wmi.ExecQuery(\"Select PercentProcessorTime From Win32_PerfFormattedData_PerfProc_Process Where IDProcess = \" & program.ProcessID)",
    "      If CLng(item.PercentProcessorTime) > 0 Then busy = True",
    "    Next",
    "    If busy Then quiet = 0 Else quiet = quiet + 1",
    "  Loop",
    "End Sub",
    "",
    "Sub Mark(name)",
    "  shell.Run \"cmd /c >COM1 echo ===SHOT \" & short & \" \" & name & \"===\", 0, True",
    "  WScript.Sleep 4000",
    "End Sub",
];

/// Gives the text of `drive.vbs`, with CRLF.
pub(crate) fn drive_script() -> String {
    crlf_text(DRIVE_LINES)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]
mod tests {
    use super::{drive_script, parse_steps, render_steps};

    #[test]
    fn the_steps_of_each_program_come_by_its_key() {
        let text = concat!(
            "[[program]]\n",
            "key = \"a/A.exe\"\n",
            "steps = [\"Hello\", '{TAB} ']\n",
            "[[program]]\n",
            "key = \"b/B.exe\"\n",
            "steps = []\n",
        );
        let steps = parse_steps(text).unwrap();
        assert_eq!(steps.get("a/A.exe").unwrap(), &["Hello", "{TAB} "]);
        assert!(steps.get("b/B.exe").unwrap().is_empty());
        assert_eq!(
            render_steps(steps.get("a/A.exe").unwrap()),
            "Hello\r\n{TAB} \r\n"
        );
    }

    #[test]
    fn a_key_twice_an_empty_step_and_a_line_break_are_refused() {
        let twice = "[[program]]\nkey = \"a\"\nsteps = []\n[[program]]\nkey = \"a\"\nsteps = []\n";
        assert!(parse_steps(twice).unwrap_err().contains("comes twice"));
        let empty = "[[program]]\nkey = \"a\"\nsteps = [\"\"]\n";
        assert!(parse_steps(empty).is_err());
        let line_break = "[[program]]\nkey = \"a\"\nsteps = [\"x\\ny\"]\n";
        assert!(parse_steps(line_break).is_err());
        let unknown = "[[program]]\nkey = \"a\"\nsteps = []\nwait = 1\n";
        assert!(parse_steps(unknown).is_err());
    }

    #[test]
    fn a_random_box_must_hold_a_pixel_and_name_a_screen_of_the_program() {
        let text = |screens: &str, corners: &str| {
            format!(
                "[[program]]\nkey = \"a\"\nsteps = [\"x\"]\nwindow = true\n\
                 random = [{{ screens = {screens}, box = {corners}, why = \"Rnd\" }}]\n"
            )
        };
        assert!(parse_steps(&text(r#"["", "s1"]"#, "[1, 2, 1, 2]")).is_ok());
        assert!(parse_steps(&text(r#"["s2"]"#, "[1, 2, 3, 4]")).is_err());
        assert!(parse_steps(&text(r#"["s0"]"#, "[1, 2, 3, 4]")).is_err());
        assert!(parse_steps(&text("[]", "[1, 2, 3, 4]")).is_err());
        assert!(parse_steps(&text(r#"[""]"#, "[3, 2, 1, 4]")).is_err());
        assert!(parse_steps(&text(r#"[""]"#, "[1, 4, 3, 2]")).is_err());
    }

    fn keys_at(text: &str) -> usize {
        text.find("shell.SendKeys keys").unwrap()
    }

    #[test]
    fn the_script_sends_a_marker_before_and_after_each_step() {
        let text = drive_script();
        assert!(!text.replace("\r\n", "").contains('\n'));
        assert!(text.contains(r#"path = files.GetAbsolutePathName("steps\" & short & ".txt")"#));
        assert!(text.contains("      shell.SendKeys keys\r\n"));
        let guard = text
            .find("If program.Status = 0 And shell.AppActivate(program.ProcessID) Then")
            .unwrap();
        assert!(guard < text.find("shell.SendKeys").unwrap());
        assert!(text.contains(r#"===NOKEYS " & short & " " & side & "-s" & count & "==="#));
        assert!(text.contains(r#"    Mark side & "-s" & count"#));
        assert!(text.contains(
            r#"shell.Run "cmd /c >COM1 echo ===SHOT " & short & " " & name & "===", 0, True"#
        ));
        let idle = text.find("    WaitIdle\r\n").unwrap();
        let marker = text.find("    Mark side & \"-s\" & count").unwrap();
        assert!(keys_at(&text) < idle && idle < marker);
        assert!(text.contains("Do While quiet < 3 And samples < 40 And program.Status = 0"));
        assert!(text.contains("Where IDProcess = \" & program.ProcessID"));
        let first = text.find("Mark side\r\n").unwrap();
        let keys = text.find("shell.SendKeys").unwrap();
        assert!(first < keys);
    }
}
