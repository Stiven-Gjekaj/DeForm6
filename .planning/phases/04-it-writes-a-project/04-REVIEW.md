---
status: issues
critical: 3
warning: 3
info: 1
files_reviewed: 15
files_reviewed_list:
  - crates/deform6/src/write/mod.rs
  - crates/deform6/src/write/model.rs
  - crates/deform6/src/write/values.rs
  - crates/deform6/src/write/vbp.rs
  - crates/deform6/src/write/frm.rs
  - crates/deform6/src/write/code.rs
  - crates/deform6/src/write/comment.rs
  - crates/deform6/src/report.rs
  - crates/deform6/src/lib.rs
  - crates/deform6-cli/src/main.rs
  - crates/xtask/src/main.rs
  - crates/deform6/tests/extract_tracer.rs
  - crates/deform6/tests/extract_structural.rs
  - crates/deform6/tests/ratios.rs
  - crates/deform6-cli/tests/cli.rs
depth: standard
---

# Phase 4 Code Review: It Writes a Project

## Summary

This review reads the write-side source files phase 4 added, against the
phase's own research, threat model, and declared gaps. The nine plans do
real, careful work: the `.frx` cursor is genuinely shared, never
re-derived; `SafeName` genuinely blocks path traversal for every form,
control, module, class, and project name; the containment check in the CLI
resolves and compares real paths, not string prefixes; and determinism is
maintained by using ordered `Vec`s everywhere a lookup table could tempt a
`HashMap`.

Three findings below are Critical. Each one is a place where a raw,
untrusted string reaches written output through a path that skips the
sanitisation this phase built everywhere else, or where the shared
`BlobCursor` can desynchronise from the bytes it is supposed to track. Two
of the three (the argument-name gap and the component file-name gap) are
the exact class of defect the review brief asked to hunt for: a route
around `SafeName`, in a file that otherwise disciplines every other string
the same way. None of the three is covered by an existing test; all three
are reachable from a hostile or merely corrupted executable, which
`AGENTS.md` states is the expected input.

Three Warnings and one Info item follow: a symlink-following overwrite
risk under `--force`, a latent (currently unreachable) ordering bug in
`SafeName`'s own legality check, a gap where procedure-level uncertainty
items never reach the comment mechanism built to surface exactly that kind
of doubt, and one report-path field that is built from a raw string
instead of a `SafeName`.

## Critical Issues

### CR-01: `append_blob` advances the shared `BlobCursor` before checking whether the blob's bytes actually fit in `data`, desynchronising every later blob's `.frx` offset from where its bytes really land

**File:** `crates/deform6/src/write/frm.rs:63-96` (the cursor advance is line 76; the bound check that can fail is lines 78-93)

**Issue:** `append_blob` calls `blob_cursor.take(&blob)?` first, which permanently advances `BlobCursor`'s internal running offset by `declared_len + FRX_ITEM_HEADER_LEN` and returns the offset this blob is assigned. Only after that does the function check whether `offset..offset + 4 + declared_len` actually fits inside `data`. When it does not, the function returns `Ok(None)` and appends nothing to `frx`, and the caller (`write_model_control_block`, lines 540-566) correctly omits the property line and records an `unrecoverable` item — but the cursor has already been advanced as if `declared_len + 4` bytes had been written to `frx`. Nothing rolls it back.

For a form with two or more resource blobs, if the byte range of an earlier one does not fit inside `data`, every blob that follows it (in the same form, same `write_form` call) is assigned an `.frx` offset that overstates its true position in the written `.frx` file by the size of the phantom advance. The `.frm` line for that later blob names an offset the real `.frx` file does not actually hold the right bytes at — the exact "offset drift" failure `crate::vb::frx::BlobCursor`'s own doc comment, and this whole phase's architecture, exists specifically to prevent (`crates/deform6/src/vb/frx.rs:8-16`: "any independent computation of an offset drifts"). Here the drift is not from a second computation; it is from one shared cursor whose state silently diverges from the buffer it is meant to describe.

`write_form`'s public signature (`pub fn write_form(form: &FormModel, defects: &[Defect], data: &[u8])`, line 675) takes an arbitrary `FormModel` and an arbitrary `data` slice — it is not restricted to `FormModel`s built by `from_report` from the same `data`. In the normal `deform6 extract` pipeline, a `PropertyValue::Blob`'s range is validated once already, at read time, by `crate::vb::frx::extract_blob` against the same underlying bytes (`crates/deform6/src/vb/frx.rs:118-224`), so this exact scenario should not arise through `inspect` → `write::project` alone. But that invariant is not enforced by any type here, is not documented as a precondition on `write_form`, and does not hold for any other caller — including this file's own test suite, which routinely builds `FormModel`/`ControlModel`/`PropertyValue::Blob` fixtures directly with hand-picked `data` (e.g. `tests::an_unreadable_blob_gives_a_report_item_and_does_not_advance_the_cursor`, lines 1062-1110). That test's own name claims the cursor "does not advance," but it only exercises `PropertyValue::BlobUnreadable` (which never reaches `append_blob` at all — `values::format_value` omits it before `collect_pending_lines` ever builds a `PendingBlob`) and a `PropertyValue::Blob` whose range happens to fit exactly. No test anywhere in this file exercises a `PropertyValue::Blob` whose range does *not* fit, so this desync path has zero coverage.

**Concrete failure:** A form (real or hostile: a truncated/damaged executable that still parses far enough to produce two `PropertyValue::Blob` records, the first with a corrupted `declared_len`/`offset` pair that no longer fits the file once re-read at write time — or any future caller that constructs a `FormModel` directly, such as a Phase 5 fuzz target) writes a `.frm` whose second `Icon`/`Picture` line names a `.frx` byte offset that does not correspond to where that blob's bytes are actually packed in the written `.frx`. VB6's IDE would read the wrong bytes, or read past the end of the file, when loading that control's picture — a silent, wrong result that exits 0 and produces a report with no defect naming it.

**Fix:** Perform the bound check first, and call `blob_cursor.take` only once the range is known to fit and the bytes are about to be appended:

```rust
fn append_blob(
    data: &[u8],
    offset: u32,
    declared_len: u32,
    blob_cursor: &mut BlobCursor,
    frx: &mut Vec<u8>,
) -> Result<Option<u32>, Refusal> {
    let Ok(start) = usize::try_from(offset) else {
        return Ok(None);
    };
    let Ok(declared) = usize::try_from(declared_len) else {
        return Ok(None);
    };
    let Some(total) = 4_usize.checked_add(declared) else {
        return Ok(None);
    };
    let Some(end) = start.checked_add(total) else {
        return Ok(None);
    };
    let Some(bytes) = data.get(start..end) else {
        return Ok(None);
    };

    let blob = frx::Blob { header: [0u8; 8], image: Vec::new(), declared_len, offset };
    let frx_offset = blob_cursor.take(&blob)?;
    frx.extend_from_slice(bytes);
    Ok(Some(frx_offset))
}
```

Add a test that builds a `FormModel` with two `PropertyValue::Blob`s where the first's range does not fit `data`, and asserts the second blob's declared `.frx` offset in the `.frm` text equals its actual byte position in the returned `.frx` buffer (not merely that a report item was produced for the first one).

---

### CR-02: A recovered argument name is written into a procedure signature with no identifier-legality check, unlike every other name this phase writes

**File:** `crates/deform6/src/write/code.rs:221-247` (`format_argument`); source of the unchecked string: `crates/deform6/src/vb/functyp.rs:754-755` (`resolve_arg_names`)

**Issue:** `format_argument` only checks whether `arg.name` is *empty* (line 222) before writing it into the generated signature line; it performs no check that the characters it contains are legal in a Visual Basic identifier. Compare this to every other recovered name this phase writes:

- A form, control, module, class or project name is always routed through `SafeName::new` (`crates/deform6/src/write/model.rs:185-265`), which replaces every character that is not a letter, digit or underscore, fixes a leading non-letter, and clamps the length.
- A procedure *name* (`ProcedureEntry::Public { name, .. }`) is validated at read time by `is_plausible_identifier` (`crates/deform6/src/vb/privateobj.rs:643-653`): it is only ever constructed as `Procedure::Public` when every byte is ASCII alphanumeric or `_` and the first is a letter or `_`.

An argument name has no equivalent check anywhere in this codebase. `resolve_arg_names` (`crates/deform6/src/vb/functyp.rs:698-762`) reads it with a plain NUL-terminated byte scan (`name_region.cstr(...)`, line 754) and decodes it with the crate's usual `char::from(byte)` convention (line 755) — any byte 0x01-0xFF except NUL is legal content for this field. `Argument.name` (`crates/deform6/src/vb/functyp.rs:241-244`) is a bare `String`, and `write::code::format_argument` writes it verbatim (`format!("{prefix}{name}")`, line 236) into the signature line `write::code::format_signature` builds.

**Concrete failure:** A crafted or corrupted executable whose `lpAryArgNames` table entry for one argument contains, say, a space, a parenthesis, or a comma produces a signature line such as `Public Sub Foo(my arg As Long)` — not legal Visual Basic, so the generated `.bas`/`.cls`/`.frm` fails to compile in the VB6 IDE, directly contradicting WRT-07's stated goal ("a project that still builds"). Worse: if the byte sequence contains a carriage return or line feed, the argument "name" injects an attacker-chosen extra line into the middle of the generated signature line, splitting it into two lines of source text the crafted executable controls the content of — a source-injection class of defect, not merely a cosmetic one. Nothing in this phase's own threat model or SUMMARY documents this as a known, accepted gap; every other place a raw string could reach written output (`write::vbp::write_quoted_setting`, `write::values::inline_decision`) explicitly checks for and refuses a line break for exactly this reason (T-4-05).

**Fix:** Sanitise `arg.name` the same way any other written identifier is sanitised before it reaches `format_argument` — either route it through a `SafeName`-style legality pass (replace an illegal character, fix a non-letter lead, generate `Arg<N>` when the result would be empty or contain nothing but replacements) or, at minimum, reuse `is_plausible_identifier`'s own check and fall back to the existing `Arg<N>` placeholder whenever it fails, not only when the name is empty.

---

### CR-03: `write::vbp::object_line` writes a component's raw recovered file name into an unquoted `.vbp` line with no line-break check, unlike every other raw field this same file writes

**File:** `crates/deform6/src/write/vbp.rs:237-255` (`object_line`, the `format!` at line 239); contrast with `crates/deform6/src/write/vbp.rs:270-290` (`write_quoted_setting`); source of the unchecked string: `crates/deform6/src/vb/project.rs:736` (`Component::file_name`), decoded by `component_cstr` (`crates/deform6/src/vb/project.rs:1181-1184`)

**Issue:** `object_line` builds an `Object=` line as `format!("Object={{{identifier}}}#1.0#0; {}", component.file_name)`. `identifier` is a hex GUID string this repository itself formats from sixteen raw bytes, so it cannot carry a line break. `component.file_name`, however, is a raw recovered `String` read by `component_cstr` with a plain NUL-terminated byte scan and the crate's usual `char::from(byte)` decoding — it can legally hold any byte 0x01-0xFF, including `\r` (0x0D) and `\n` (0x0A). Nothing in `object_line`, nor anywhere upstream in `crates/deform6/src/vb/project.rs`, checks its content.

This is exactly the T-4-05 threat this same file already defends against for `Title`, `HelpFile`, `ExeName32`, and `Name`: `write_quoted_setting` (lines 270-290) explicitly checks `value.contains('\r') || value.contains('\n')` and refuses (writing an empty value and a report item) rather than writing a line break into a `.vbp` line. `object_line` is the one place in this same file that writes a raw, unsanitised recovered string into a `.vbp` line without that same check. The doc comment on `object_line` (lines 233-236) explains, correctly, why the file name is not routed through `SafeName` (it names a file this phase does not create, so path-safety rules do not apply) — but that reasoning only covers *why no `SafeName` clamp is needed*; it does not address, and the code does not implement, the separate line-integrity check `write_quoted_setting` performs for every other raw string in this file.

**Concrete failure:** A declared external component whose file-name field (read straight from the PE's component table) contains an embedded CR or LF produces a `.vbp` with an extra, attacker-chosen line inserted in the middle of the `Object=` line — for example, a crafted file name of `"MSWINSCK.OCX\r\nStartup=\"Sub Main\""` would inject a second `Startup=` key (or any other key) into the generated project file, silently changing what the reconstructed project declares, with the run still exiting 0 and no defect recorded.

**Fix:** Apply the same refusal `write_quoted_setting` uses: if `component.file_name` contains `\r` or `\n`, omit the `Object=` line entirely (or write it with an empty file name) and add an `unrecoverable` report item naming the reason, mirroring the existing pattern instead of introducing a second one:

```rust
fn object_line(component: &Component) -> Option<(String, ReportItem)> {
    let identifier = component.ouuid_text.as_ref()?;
    if component.file_name.contains('\r') || component.file_name.contains('\n') {
        // refuse, as write_quoted_setting already does for every other field
        return None;
    }
    let line = format!("Object={{{identifier}}}#1.0#0; {}", component.file_name);
    // ...
}
```
(Adjust the caller in `write_components` so a refusal here still produces a report item, the same way a component with no resolved identifier already does.)

## Warnings

### WR-01: `--force` overwrite writes through a pre-existing symlink at the target path, with no check that the entry being replaced is a plain file

**File:** `crates/deform6-cli/src/main.rs:466-493` (`write_project`), specifically the `std::fs::write(path, bytes)` call at line 489; `ensure_directory_is_writable` (lines 346-367) and `plan_writes` (lines 405-429)

**Issue:** `plan_writes` validates only the *logical* shape of each candidate path (no `..`, no absolute component, a direct child of the resolved output directory) — a correctness property this review's `<what_to_weigh_heavily>` list explicitly asked to check, and which is genuinely well handled for that threat. It never checks whether an entry already exists at that exact path and, if so, whether that entry is itself a symlink. `ensure_directory_is_writable` bypasses its own non-empty-directory refusal entirely when `--force` is given (line 347-349), and the write loop that follows calls `std::fs::write`, which follows an existing symlink on Unix rather than replacing it.

**Concrete failure:** If a user runs `deform6 extract some.exe -o /shared/dir --force` and `/shared/dir` is a directory another local user (or process) has write access to, that other party can pre-plant a symlink at one of this tool's predictable output file names (e.g. `Project1.vbp`, or `<ProjectName>.report.json`) pointing at an arbitrary file the running user can write (e.g. a dotfile in their home directory). The extract run silently overwrites the symlink's target instead of creating a new file inside `/shared/dir` — a write outside the resolved output directory the containment check was built specifically to prevent, achieved without needing a hostile executable at all.

**Fix:** Before writing each planned file, check with `std::fs::symlink_metadata` whether an entry already exists at that path and is a symlink (or, on platforms that support it, open with `O_NOFOLLOW`/`create_new` and remove-then-recreate on conflict), and refuse the whole run (as every other containment failure already does) rather than following it.

### WR-02: `SafeName`'s identifier-legality check runs before the encoder that can still turn a "legal" character into `?`, an illegal one

**File:** `crates/deform6/src/write/model.rs:358-360` (`is_legal_identifier_char`), used inside `SafeName::new` at lines 222-235, with the encoding/clamp step at lines 249-256

**Issue:** `is_legal_identifier_char` accepts `ch.is_alphanumeric()`, which is true for any Unicode letter or digit, not only the Latin-1 range (0x00-0xFF) this crate's own read/write convention is built around. A character survives the legality pass unchanged as long as it is alphanumeric, and only afterward, inside `encode_windows_1252` (called at line 249), does a character above `0xFF` get replaced with a literal `?` (`crates/deform6/src/write/model.rs:56-62`). Because that substitution happens after the legality check, not as part of it, the final decoded `value` (line 256) can contain `?` — a character that is neither a legal VB6 identifier character (the property this function exists to guarantee) nor a legal Windows file-name character.

This is very likely unreachable through the actual `deform6 extract` pipeline today: every string `crate::vb::Report` carries is itself decoded with `char::from(byte)` (`bytes.iter().copied().map(char::from).collect()`, the same convention `crates/deform6/src/vb/vbstr.rs:245` documents), which by construction can never produce a character above `U+00FF`. So no name `from_report` builds from a real `Report` can ever reach the `> 0xFF` branch of `encode_windows_1252` at all. But that is an invariant maintained several modules away, not something `SafeName::new` itself asserts or is documented to depend on, and `SafeName::new` is a `pub fn` any future caller (including a fuzz harness) can call with an arbitrary `&str`.

**Fix:** Make `is_legal_identifier_char` itself reject anything above `0xFF` (`ch.is_alphanumeric() && (ch as u32) <= 0xFF`), so the legality guarantee holds unconditionally rather than depending on an invariant enforced elsewhere, and add a test that calls `SafeName::new` directly with a character above `U+00FF` and asserts the result never contains `?`.

### WR-03: Procedure-level uncertainty items never reach an uncertainty comment, only the JSON report — even though the comment mechanism exists specifically to surface this class of doubt

**File:** `crates/deform6/src/write/code.rs:460-469` (`write_code_region`)

**Issue:** `write_code_region` calls `uncertainty_comments(items, path_prefix)` using only the `items` parameter its caller supplied, then separately calls `format_procedures(procedures)`, which produces its own `procedure_items` (an "inferred, no recovered prototype" item for every public procedure in a standard module, an "unrecoverable, generated name" item for every private slot, an "unrecoverable, no name array at all" item for an object with none — `crates/deform6/src/write/code.rs:337-375, 388-410`). Those `procedure_items` are only ever *returned* by `write_code_region` (as part of the function's own result), never fed back into the `uncertainty_comments` call that already ran inside the same function. Every caller (`write_cls`, `write_bas`, and `write::frm::write_form`) accumulates `procedure_items`/`code_items` into its own report-item list for the JSON report, but none of them makes a second pass to turn those specific items into comment lines in the file that was just written.

The practical effect: every `.bas` standard module in the corpus (every one of which has "no recovered prototype" for all of its procedures, per this plan's own SUMMARY: "which happens for every procedure in a standard module by construction") writes zero comments about that fact. The doubt is real, is recorded faithfully in the JSON report, but never appears in the file `write::comment`'s own module doc comment says is "the one place doubt about a recovered fact can become a line in a written file."

**Concrete failure:** This is not incorrect VB6 (the file still loads), and the JSON report is not wrong — so this is a completeness gap in the diagnostic feature, not a data-loss bug. It is worth flagging because the corpus-wide sweep test (`write::code::sweep`, per the 04-07 SUMMARY) only asserts "at least one program produces at least one comment below the boundary somewhere," an assertion easily satisfied by unrelated control/property-level items (Undecoded properties are the majority case, 683 of 807 records) and therefore incapable of catching that an entire category of item (every procedure-level item, in every file) never produces a comment anywhere.

**Fix:** Have `write_code_region` fold its own `procedure_items` into the same `uncertainty_comments` call, or make a second call over just those items and append the resulting lines, so a reader opening a `.bas` file sees the same doubt the JSON report already states about its own procedures.

## Info

### IN-01: A component's report path is built from a raw recovered string, not a `SafeName`

**File:** `crates/deform6/src/write/vbp.rs:189, 241` (`format!("/components/{}", component.name)`)

**Issue:** Every other path this phase's report items carry is built through `crate::report::path_for_form`/`path_for_control`/`path_for_code`, each of which takes a `&SafeName` (`crates/deform6/src/report.rs:132-175`), matching the design stated there: "every name segment comes from a `SafeName`, never from a raw recovered string." A component's own path (used only as a `ReportItem.path` value in the JSON report, never as a file path) is instead built directly from `component.name`, a raw `String` (`crates/deform6/src/vb/project.rs:740`) that can contain a `/` or other structurally significant character, since `component_cstr` applies no restriction.

**Concrete failure:** Low risk in itself — this path never reaches the file system, only the JSON report — but a component name containing `/` produces a `path` value that looks like several nested path segments in the flat report array, inconsistent with every other item's path shape, and `PathIssuer`'s own collision handling (`crates/deform6/src/report.rs:191-218`) was not designed with this case in mind.

**Fix:** Route `component.name` through a lightweight `SafeName`-style sanitisation (even one that only strips `/` and control characters) before it is embedded in a report path, for consistency with the rest of this file's own design rule.

---

_Reviewed: 2026-09-13T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
