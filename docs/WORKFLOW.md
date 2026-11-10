# From a compiled program to a rebuilt one

This file gives one path. You start with a Visual Basic 6 executable and no
source. You end with a project that the Visual Basic 6 IDE opens, edits and
builds.

DeForm6 does not recover the source of a procedure. For a P-code program,
`--lift` writes the P-code of each procedure as statements. The statements
name the variables by their offsets, such as `local_88`. For a native
program, each procedure body stays empty, and you write the logic.

## What you need

- The Rust toolchain that the repository pins, 1.97.1.
- A copy of `VB6.OLB` and of `MSVBVM60.DLL` that you own. The Visual Basic 6
  IDE installs both.
- For the build step, a Windows host with the Visual Basic 6 IDE. DeForm6
  runs on Windows 10 or later. It does not run on Windows XP. The setup of
  VB6 starts with a 16-bit program, which a 64-bit Windows does not run.
  Start `Setup\acmsetup.exe /T VB98ENT.STF /S <drive>:\` from the VB6
  disc instead, and clear **Data Access** in a custom install.

## Step 1: make the tables, one time

The tables come from the Microsoft files that you own. The repository does
not hold them. Run these commands one time, on any host:

```
cargo run -p xtask -- derive-opcode-table <path-to-VB6.OLB>
cargo run -p xtask -- derive-vb-types <path-to-VB6.OLB> <path-to-MSVBVM60.DLL> [<path-to-OCX>...]
cargo run -p xtask -- fetch-pcode-symbols <path-to-MSVBVM60.DLL>
cargo run -p xtask -- derive-pcode-table <path-to-MSVBVM60.DLL> derived/symbols/MSVBVM60.pdb
```

Each command writes its file into `derived/`. Give each OCX that your
program uses to `derive-vb-types`, such as `MSWINSCK.OCX`. The file then
holds the type library of each control, and `extract --vb-types` writes its
`Object=` line.

`fetch-pcode-symbols` reads the Microsoft symbol server. Run it on a host
with a network. The other commands do not need a network.

## Step 2: write the project

For a P-code program:

```
deform6 extract <path-to-exe> -o <output-directory> \
    --opcode-table derived/opcode-table.toml \
    --vb-types derived/vb-types.toml \
    --pcode-table derived/pcode-table.toml --lift
```

For a native program, leave out `--pcode-table` and `--lift`. To find which
one you have, run `deform6 inspect <path-to-exe>`. Its `Mode` line gives
`p-code` or `native`.

The output directory holds the `.vbp` file, a `.frm` and a `.frx` file for
each form, a `.bas` file for each module, a `.cls` file for each class, and
the JSON report. The report gives the confidence of each item:
`proven`, `inferred` or `unrecoverable`. Read the `inferred` items first.

## Step 3: build it

On the Windows host, add `--verify-build` to the command of step 2:

```
deform6 extract <path-to-exe> -o <output-directory> <the flags of step 2> --verify-build
```

DeForm6 finds `VB6.EXE` in this order:

1. The path that `--compiler` gives.
2. The path that the environment variable `DEFORM6_VB6` gives.
3. `Microsoft Visual Studio\VB98\VB6.EXE` under `ProgramFiles(x86)`, then
   under `ProgramFiles`.

It runs `VB6.EXE /make` on the written project. The executable goes into
`<output-directory>\build`. The report gets the key `build`, with the
compiler, the outcome, the exit code and the lines that VB6 wrote.

| Outcome | Exit code of `deform6` | What to do next |
|---|---|---|
| `built` | 0 | Go to step 4. |
| `built with load errors` | 6 | VB6 could not load a control class, and it put a picture box in its place. The messages name the `.log` file and the control. Make sure that the OCX of the control is registered on the host. Then compare the `Object=` line of the `.vbp` with the line that VB6 writes when you add the same component to a new project, and copy that line. |
| `failed` | 6 | The messages give the file and the line that VB6 refused. Open the project in the IDE and correct that line. If the message is `Error accessing the system registry`, run the same command again: on the author's Windows 11 host, this error came once and did not come back. |
| `timed out` | 6 | A dialog of the IDE stopped the build. Open the project in the IDE to see the dialog. |
| `not run` | 5 | DeForm6 found no `VB6.EXE`. Give its path with `--compiler`. |

## Step 4: edit and build in the IDE

Open the `.vbp` file in the Visual Basic 6 IDE. Edit the code, then use
**File, Make** to build the executable. This step ran on one native and one
lifted P-code project: a new event handler built and ran in each one.

For a P-code program, the project holds `CompilationType=-1`, so VB6 builds
P-code again, as the original was built. For a native program, the project
holds `CompilationType=0`, and VB6 builds native code.

A variable keeps the name of its offset, such as `local_88`, apart from the
public variables. Give each one a name when you know what it holds. Most of
the names of the source are in no byte of the executable, so no tool can
give them back.

## twinBASIC

This project has not tested twinBASIC. Its IDE imports a `.vbp` file into a
new twinBASIC project. `--verify-build` runs the Visual Basic 6 IDE only.
The published examples of its command line build a `.twinproj` file, so
import the project in the IDE first. An unattended build on the twinBASIC
command line needs its Professional Edition.

## What this path does not give

- The source of a procedure. The lift is the P-code, written as statements.
- Statements for a native program.
- The names of most variables.
- The `Object=` line of a control other than the Winsock control, when you
  do not give its OCX to `derive-vb-types`. DeForm6 then writes the class
  identifier on that line, and VB6 does not load it.
