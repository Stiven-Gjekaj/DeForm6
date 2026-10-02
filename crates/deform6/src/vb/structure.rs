//! Blocks of Basic in place of `GoTo` and labels.
//!
//! The lift gives each body as a list of statements, with a branch as a
//! `GoTo` and a label. This module finds the shapes that Basic compiles
//! from a block, and writes them as that block:
//!
//! - `If Not c Then GoTo T`, statements, `T:` is `If c Then`, the
//!   statements, `End If`.
//! - When the statements end with `GoTo X`, and `X:` comes after more
//!   statements that follow `T:`, the shape is `If c Then`, `Else`,
//!   `End If`.
//! - `T: If Not c Then GoTo X`, statements, `GoTo T`, `X:` is `Do While c`,
//!   the statements, `Loop`.
//!
//! A shape becomes a block only when no branch from outside the block lands
//! inside it, and when each `For` and each `Next` inside it has its partner
//! inside it too. Each other branch stays a `GoTo`, and its label stays.

use crate::vb::lift::{BinaryOp, Expr, LiftedStmt, Stmt};

/// One node of a structured body.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Node {
    /// The statement at an index, as the lift gives it.
    Plain(usize),
    /// `If`, from the branch at `index`: `then` runs when the branch does
    /// not go, and `otherwise`, when there is one, runs when it goes.
    If {
        /// The index of the branch.
        index: usize,
        /// The statements that run when the branch does not go.
        then: Vec<Node>,
        /// The index of the `GoTo` at the end of `then`, which `Else` takes
        /// the place of, and the statements that run when the branch goes.
        otherwise: Option<(usize, Vec<Node>)>,
    },
    /// `Do While`, from the branch at `index` at the top of the loop.
    While {
        /// The index of the branch.
        index: usize,
        /// The index of the `GoTo` back to the top, which `Loop` takes the
        /// place of.
        back: usize,
        /// The statements of the loop.
        body: Vec<Node>,
    },
}

/// The facts of a body that the shapes need.
struct Body<'a> {
    stmts: &'a [LiftedStmt],
    /// The index of the statement that each branch target names, or the
    /// length of the body for a target after the last statement.
    targets: Vec<(u16, usize)>,
}

impl<'a> Body<'a> {
    fn new(stmts: &'a [LiftedStmt]) -> Self {
        let mut targets = Vec::new();
        for stmt in stmts {
            for target in branch_targets(&stmt.stmt) {
                let index = stmts
                    .iter()
                    .position(|other| positions(other).contains(&target))
                    .unwrap_or(stmts.len());
                if !targets.contains(&(target, index)) {
                    targets.push((target, index));
                }
            }
        }
        Self { stmts, targets }
    }

    fn stmt(&self, index: usize) -> Option<&'a Stmt> {
        self.stmts.get(index).map(|lifted| &lifted.stmt)
    }

    /// The index of the statement that `target` names.
    fn index_of(&self, target: u16) -> usize {
        self.targets
            .iter()
            .find(|(at, _)| *at == target)
            .map_or(self.stmts.len(), |(_, index)| *index)
    }

    /// The number of branches of the body that name `target`.
    fn uses(&self, target: u16) -> usize {
        self.stmts
            .iter()
            .flat_map(|lifted| branch_targets(&lifted.stmt))
            .filter(|at| *at == target)
            .count()
    }

    /// The number of branches of the body that name a position of the
    /// statement at `index`.
    fn uses_of(&self, index: usize) -> usize {
        self.stmts.get(index).map_or(0, |lifted| {
            positions(lifted).iter().map(|at| self.uses(*at)).sum()
        })
    }

    /// Tells whether the statements `lo..hi` can be a block: no branch from
    /// outside lands inside them, apart from the branch at `entry`, and each
    /// `For` and `Next` among them has its partner among them.
    fn is_block(&self, lo: usize, hi: usize, entry: Option<usize>) -> bool {
        for (at, lifted) in self.stmts.iter().enumerate() {
            if entry == Some(at) {
                continue;
            }
            let inside = (lo..hi).contains(&at);
            for target in branch_targets(&lifted.stmt) {
                let index = self.index_of(target);
                match &lifted.stmt {
                    Stmt::For { .. } if inside => {
                        if !(lo < index && index <= hi) {
                            return false;
                        }
                    }
                    Stmt::Next { .. } if inside => {
                        if !(lo <= index && index <= at) {
                            return false;
                        }
                    }
                    _ if !inside && (lo..hi).contains(&index) => return false,
                    _ => {}
                }
            }
        }
        true
    }

    /// Gives the nodes of the statements `lo..hi`.
    fn nodes(&self, lo: usize, hi: usize) -> Vec<Node> {
        let mut out = Vec::new();
        let mut at = lo;
        while at < hi {
            if let Some((node, next)) = self.loop_at(at, hi).or_else(|| self.if_at(at, hi)) {
                out.push(node);
                at = next;
            } else {
                out.push(Node::Plain(at));
                at = at.saturating_add(1);
            }
        }
        out
    }

    /// The `Do While` loop whose test is the statement at `at`, and the
    /// index after it.
    fn loop_at(&self, at: usize, hi: usize) -> Option<(Node, usize)> {
        let Some(Stmt::IfNotGoTo { target, .. }) = self.stmt(at) else {
            return None;
        };
        let end = self.index_of(*target);
        let back = end.checked_sub(1)?;
        if end > hi || back <= at {
            return None;
        }
        let Some(Stmt::GoTo(top)) = self.stmt(back) else {
            return None;
        };
        let top_here = self
            .stmts
            .get(at)
            .is_some_and(|lifted| positions(lifted).contains(top));
        let first = at.checked_add(1)?;
        if !top_here || self.uses_of(at) != 1 || !self.is_block(first, back, None) {
            return None;
        }
        Some((
            Node::While {
                index: at,
                back,
                body: self.nodes(first, back),
            },
            end,
        ))
    }

    /// The `If` block whose branch is the statement at `at`, and the index
    /// after it.
    fn if_at(&self, at: usize, hi: usize) -> Option<(Node, usize)> {
        let target = match self.stmt(at)? {
            Stmt::IfNotGoTo { target, .. } | Stmt::IfGoTo { target, .. } => *target,
            _ => return None,
        };
        let first = at.checked_add(1)?;
        let end = self.index_of(target);
        if end <= first || end > hi || !self.is_block(first, end, None) {
            return None;
        }
        if let Some(node) = self.if_else(at, first, end, target, hi) {
            return Some(node);
        }
        Some((
            Node::If {
                index: at,
                then: self.nodes(first, end),
                otherwise: None,
            },
            end,
        ))
    }

    /// The `If` block with an `Else`: the statements before `end` end with
    /// `GoTo X`, and `X` comes after more statements.
    fn if_else(
        &self,
        at: usize,
        first: usize,
        end: usize,
        target: u16,
        hi: usize,
    ) -> Option<(Node, usize)> {
        let last = end.checked_sub(1)?;
        let Some(Stmt::GoTo(after)) = self.stmt(last) else {
            return None;
        };
        let stop = self.index_of(*after);
        if last < first
            || stop <= end
            || stop > hi
            || self.uses(target) != 1
            || !self.is_block(first, last, None)
            || !self.is_block(end, stop, Some(at))
        {
            return None;
        }
        Some((
            Node::If {
                index: at,
                then: self.nodes(first, last),
                otherwise: Some((last, self.nodes(end, stop))),
            },
            stop,
        ))
    }
}

/// Gives the offsets that a branch can name for `lifted`: its own offset
/// and the offsets of the opcodes with no effect before it.
fn positions(lifted: &LiftedStmt) -> Vec<u16> {
    lifted
        .also_at
        .iter()
        .chain(std::iter::once(&lifted.offset))
        .filter_map(|offset| u16::try_from(*offset).ok())
        .collect()
}

/// Gives each offset that `stmt` can go to.
fn branch_targets(stmt: &Stmt) -> Vec<u16> {
    match stmt {
        Stmt::IfNotGoTo { target, .. }
        | Stmt::IfGoTo { target, .. }
        | Stmt::GoTo(target)
        | Stmt::For { exit: target, .. }
        | Stmt::Next { body: target, .. } => vec![*target],
        Stmt::OnError(Some(target)) | Stmt::Resume(Some(target)) if *target != 0 => {
            vec![*target]
        }
        _ => Vec::new(),
    }
}

/// Gives the condition of an `If` or a `Do While` that runs its block when
/// the branch of `stmt` does not go.
fn condition(stmt: &Stmt) -> Option<String> {
    match stmt {
        Stmt::IfNotGoTo { condition, .. } => Some(condition.text()),
        Stmt::IfGoTo { condition, .. } => Some(match condition {
            Expr::Binary(
                BinaryOp::Lt
                | BinaryOp::Gt
                | BinaryOp::Eq
                | BinaryOp::Ne
                | BinaryOp::Le
                | BinaryOp::Ge,
                _,
                _,
            ) => format!("Not {}", condition.text()),
            _ => format!("Not CBool({})", condition.text()),
        }),
        _ => None,
    }
}

/// The text before a statement at the depth `depth`.
fn indent(depth: usize) -> String {
    format!("       {}", "    ".repeat(depth))
}

/// The text of `stmt`. A `For` and a `Next` are Basic with no comment: the
/// loop needs no label, and [`live_targets`] gives none for it.
fn plain_text(stmt: &Stmt) -> String {
    match stmt {
        Stmt::For {
            counter,
            start,
            end,
            step,
            ..
        } => {
            let step = step
                .as_ref()
                .map(|step| format!(" Step {}", step.text()))
                .unwrap_or_default();
            format!(
                "For {} = {} To {}{step}",
                counter.text(),
                start.text(),
                end.text()
            )
        }
        Stmt::Next { counter, .. } => format!("Next {}", counter.text()),
        other => other.text(),
    }
}

/// The labels that the statement at `index` takes: each of its positions
/// that a branch in `live` names, once in the body.
fn labels(index: usize, body: &Body<'_>, live: &[u16], placed: &mut Vec<u16>) -> Vec<u16> {
    let Some(lifted) = body.stmts.get(index) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for at in positions(lifted) {
        if live.contains(&at) && !placed.contains(&at) {
            placed.push(at);
            out.push(at);
        }
    }
    out
}

/// Tells whether the statement at `index` takes a label that is not
/// written yet.
fn has_label(index: usize, body: &Body<'_>, live: &[u16], placed: &[u16]) -> bool {
    body.stmts.get(index).is_some_and(|lifted| {
        positions(lifted)
            .iter()
            .any(|at| live.contains(at) && !placed.contains(at))
    })
}

/// Writes `nodes` at the depth `depth` into `out`, with a label for each
/// position that a branch in `live` names. `placed` holds the labels that
/// are written already.
fn write(
    nodes: &[Node],
    depth: usize,
    body: &Body<'_>,
    live: &[u16],
    placed: &mut Vec<u16>,
    out: &mut Vec<String>,
) {
    for node in nodes {
        let index = match node {
            Node::Plain(index) | Node::If { index, .. } | Node::While { index, .. } => *index,
        };
        let Some(lifted) = body.stmts.get(index) else {
            continue;
        };
        let text = match node {
            Node::Plain(_) => plain_text(&lifted.stmt),
            Node::If { .. } => format!("If {} Then", condition(&lifted.stmt).unwrap_or_default()),
            Node::While { .. } => {
                format!("Do While {}", condition(&lifted.stmt).unwrap_or_default())
            }
        };
        let own = labels(index, body, live, placed);
        match own.as_slice() {
            [only] if depth == 0 && *only == u16::try_from(lifted.offset).unwrap_or(0) => {
                out.push(format!("L{only:04X}: {text}"));
            }
            _ => {
                out.extend(own.iter().map(|at| format!("L{at:04X}:")));
                out.push(format!("{}{text}", indent(depth)));
            }
        }
        let inner = depth.saturating_add(1);
        match node {
            Node::Plain(_) => {}
            Node::If {
                then, otherwise, ..
            } => {
                write(then, inner, body, live, placed, out);
                let mut rest = otherwise.as_ref();
                while let Some((last, nodes)) = rest {
                    let at_end = labels(*last, body, live, placed);
                    out.extend(at_end.iter().map(|at| format!("L{at:04X}:")));
                    // An `Else` that holds one `If` and no label is `ElseIf`.
                    if let [
                        Node::If {
                            index: next,
                            then,
                            otherwise,
                        },
                    ] = nodes.as_slice()
                        && !has_label(*next, body, live, placed)
                        && let Some(stmt) = body.stmt(*next)
                    {
                        out.push(format!(
                            "{}ElseIf {} Then",
                            indent(depth),
                            condition(stmt).unwrap_or_default()
                        ));
                        write(then, inner, body, live, placed, out);
                        rest = otherwise.as_ref();
                        continue;
                    }
                    out.push(format!("{}Else", indent(depth)));
                    write(nodes, inner, body, live, placed, out);
                    rest = None;
                }
                out.push(format!("{}End If", indent(depth)));
            }
            Node::While {
                back, body: nodes, ..
            } => {
                write(nodes, inner, body, live, placed, out);
                let at_end = labels(*back, body, live, placed);
                out.extend(at_end.iter().map(|at| format!("L{at:04X}:")));
                out.push(format!("{}Loop", indent(depth)));
            }
        }
    }
}

/// Gives each branch target of the `Plain` nodes of `nodes`: the branches
/// that stay a `GoTo`, an `On Error` or a `Resume`. A `For` and a `Next`
/// need no label.
fn live_targets(nodes: &[Node], body: &Body<'_>, out: &mut Vec<u16>) {
    for node in nodes {
        match node {
            Node::Plain(index) => match body.stmt(*index) {
                Some(Stmt::For { .. } | Stmt::Next { .. }) | None => {}
                Some(stmt) => out.extend(branch_targets(stmt)),
            },
            Node::If {
                then, otherwise, ..
            } => {
                live_targets(then, body, out);
                if let Some((_, otherwise)) = otherwise {
                    live_targets(otherwise, body, out);
                }
            }
            Node::While { body: nodes, .. } => live_targets(nodes, body, out),
        }
    }
}

/// Gives the text of `stmts` with each shape that this module finds as a
/// block, and each other branch as a `GoTo` and a label.
#[must_use]
pub fn render(stmts: &[LiftedStmt]) -> Vec<String> {
    let body = Body::new(stmts);
    let nodes = body.nodes(0, stmts.len());
    let mut live = Vec::new();
    live_targets(&nodes, &body, &mut live);
    live.sort_unstable();
    live.dedup();
    let mut out = Vec::new();
    let mut placed = Vec::new();
    write(&nodes, 0, &body, &live, &mut placed, &mut out);
    out.extend(
        live.iter()
            .filter(|target| !placed.contains(target))
            .map(|target| format!("L{target:04X}:")),
    );
    out
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "a test builds the state it needs and must fail loudly when that state is wrong"
)]
mod tests {
    use super::render;
    use crate::vb::lift::{BinaryOp, Expr, LiftedStmt, Stmt};

    fn at(offset: u32, stmt: Stmt) -> LiftedStmt {
        LiftedStmt {
            offset,
            also_at: Vec::new(),
            stmt,
        }
    }

    fn set(slot: u16, value: i64) -> Stmt {
        Stmt::Assign {
            target: Expr::Local(slot),
            value: Expr::Const(value),
        }
    }

    fn unless(target: u16) -> Stmt {
        Stmt::IfNotGoTo {
            condition: Expr::Local(0x88),
            target,
        }
    }

    const EXIT: Stmt = Stmt::Exit { function: false };

    #[test]
    fn a_branch_over_statements_is_an_if_block() {
        let body = [at(0, unless(0x10)), at(5, set(0x8C, 1)), at(0x10, EXIT)];
        assert_eq!(
            render(&body),
            [
                "       If local_88 Then",
                "           local_8C = 1",
                "       End If",
                "       Exit Sub"
            ]
        );
    }

    #[test]
    fn a_branch_over_statements_that_end_with_a_goto_is_an_if_else_block() {
        let body = [
            at(0, unless(0x10)),
            at(5, set(0x8C, 1)),
            at(0x0A, Stmt::GoTo(0x18)),
            at(0x10, set(0x8C, 2)),
            at(0x18, EXIT),
        ];
        assert_eq!(
            render(&body),
            [
                "       If local_88 Then",
                "           local_8C = 1",
                "       Else",
                "           local_8C = 2",
                "       End If",
                "       Exit Sub"
            ]
        );
    }

    #[test]
    fn a_test_at_the_top_and_a_goto_back_is_a_do_while_loop() {
        let body = [
            at(0, set(0x8C, 0)),
            at(8, unless(0x20)),
            at(0x0E, set(0x8C, 1)),
            at(0x14, Stmt::GoTo(8)),
            at(0x20, EXIT),
        ];
        assert_eq!(
            render(&body),
            [
                "       local_8C = 0",
                "       Do While local_88",
                "           local_8C = 1",
                "       Loop",
                "       Exit Sub"
            ]
        );
    }

    #[test]
    fn a_block_with_a_branch_from_outside_into_it_stays_a_goto() {
        let body = [
            at(0, unless(0x10)),
            at(5, set(0x8C, 1)),
            at(0x10, set(0x8C, 2)),
            at(0x15, Stmt::GoTo(5)),
        ];
        assert_eq!(
            render(&body),
            [
                "       If Not local_88 Then GoTo L0010",
                "L0005: local_8C = 1",
                "L0010: local_8C = 2",
                "       GoTo L0005"
            ]
        );
    }

    #[test]
    fn a_for_loop_that_crosses_the_block_stays_a_goto() {
        let counter = Expr::Local(0x90);
        let body = [
            at(
                0,
                Stmt::For {
                    counter: counter.clone(),
                    start: Expr::Const(0),
                    end: Expr::Const(9),
                    step: None,
                    exit: 0x20,
                },
            ),
            at(8, unless(0x20)),
            at(0x0E, set(0x8C, 1)),
            at(0x14, Stmt::Next { counter, body: 8 }),
            at(0x20, EXIT),
        ];
        assert_eq!(
            render(&body),
            [
                "       For local_90 = 0 To 9",
                "       If Not local_88 Then GoTo L0020",
                "       local_8C = 1",
                "       Next local_90",
                "L0020: Exit Sub"
            ]
        );
    }

    /// `isRegKey` of `PassGen.exe` tests four conditions in a row: each
    /// `Else` holds one `If` only.
    #[test]
    fn an_else_that_holds_one_if_is_an_else_if() {
        let body = [
            at(0, unless(0x10)),
            at(5, set(0x8C, 1)),
            at(0x0A, Stmt::GoTo(0x30)),
            at(0x10, unless(0x20)),
            at(0x15, set(0x8C, 2)),
            at(0x1A, Stmt::GoTo(0x30)),
            at(0x20, set(0x8C, 3)),
            at(0x30, EXIT),
        ];
        assert_eq!(
            render(&body),
            [
                "       If local_88 Then",
                "           local_8C = 1",
                "       ElseIf local_88 Then",
                "           local_8C = 2",
                "       Else",
                "           local_8C = 3",
                "       End If",
                "       Exit Sub"
            ]
        );
    }

    /// `GoTo L00B6` of the main loop of `Physics_Demo.exe` goes to the
    /// `GoTo` back to the top of its loop, which `Loop` takes the place of.
    /// The label goes before `Loop`.
    #[test]
    fn a_branch_to_the_goto_that_a_block_takes_keeps_its_label() {
        let body = [
            at(0, unless(0x40)),
            at(5, unless(0x30)),
            at(0x0A, unless(0x20)),
            at(0x0F, Stmt::GoTo(0x38)),
            at(0x20, set(0x8C, 1)),
            at(0x28, Stmt::GoTo(5)),
            at(0x30, set(0x8C, 2)),
            at(0x38, Stmt::GoTo(0)),
            at(0x40, EXIT),
        ];
        assert_eq!(
            render(&body),
            [
                "       Do While local_88",
                "           Do While local_88",
                "               If local_88 Then",
                "                   GoTo L0038",
                "               End If",
                "               local_8C = 1",
                "           Loop",
                "           local_8C = 2",
                "L0038:",
                "       Loop",
                "       Exit Sub"
            ]
        );
    }

    /// Two statements can name the same offset, as an opcode with no effect
    /// before the second. The label goes before the first, once.
    #[test]
    fn a_label_is_written_once() {
        let mut second = at(0x10, set(0x8C, 2));
        second.also_at = vec![0x0C];
        let mut first = at(0x0C, set(0x8C, 1));
        first.also_at = vec![0x10];
        let body = [at(0, Stmt::GoTo(0x10)), first, second];
        let lines = render(&body);
        assert_eq!(
            lines
                .iter()
                .filter(|line| line.starts_with("L0010"))
                .count(),
            1
        );
    }

    #[test]
    fn a_branch_when_true_gives_the_block_its_negation() {
        let compare = Stmt::IfGoTo {
            condition: Expr::Binary(
                BinaryOp::Eq,
                Box::new(Expr::Local(0x88)),
                Box::new(Expr::Const(1)),
            ),
            target: 0x10,
        };
        let plain = Stmt::IfGoTo {
            condition: Expr::Local(0x88),
            target: 0x10,
        };
        let first = |stmt: Stmt| {
            render(&[at(0, stmt), at(5, set(0x8C, 1)), at(0x10, EXIT)])
                .first()
                .unwrap()
                .clone()
        };
        assert_eq!(first(compare), "       If Not (local_88 = 1) Then");
        assert_eq!(first(plain), "       If Not CBool(local_88) Then");
    }
}
