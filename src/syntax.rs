//! Simple Verilog / SystemVerilog-ish token highlighter.

use crate::theme;
use egui::Color32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Text,
    Keyword,
    Type,
    Number,
    String,
    Comment,
    Macro,
}

impl TokenKind {
    pub fn color(self) -> Color32 {
        match self {
            TokenKind::Text => theme::IDENT,
            TokenKind::Keyword => theme::KW,
            TokenKind::Type => theme::TYPE,
            TokenKind::Number => theme::NUM,
            TokenKind::String => theme::STR,
            TokenKind::Comment => theme::COMMENT,
            TokenKind::Macro => theme::MACRO,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub kind: TokenKind,
}

const KEYWORDS: &[&str] = &[
    "module",
    "endmodule",
    "input",
    "output",
    "inout",
    "wire",
    "reg",
    "logic",
    "integer",
    "real",
    "parameter",
    "localparam",
    "assign",
    "always",
    "always_ff",
    "always_comb",
    "always_latch",
    "initial",
    "begin",
    "end",
    "if",
    "else",
    "case",
    "casex",
    "casez",
    "endcase",
    "default",
    "for",
    "while",
    "repeat",
    "forever",
    "function",
    "endfunction",
    "task",
    "endtask",
    "generate",
    "endgenerate",
    "genvar",
    "posedge",
    "negedge",
    "or",
    "and",
    "not",
    "xor",
    "nand",
    "nor",
    "xnor",
    "buf",
    "pullup",
    "pulldown",
    "tri",
    "tri0",
    "tri1",
    "supply0",
    "supply1",
    "signed",
    "unsigned",
    "automatic",
    "typedef",
    "struct",
    "enum",
    "union",
    "packed",
    "interface",
    "endinterface",
    "modport",
    "package",
    "endpackage",
    "import",
    "export",
    "class",
    "endclass",
    "extends",
    "virtual",
    "static",
    "const",
    "return",
    "break",
    "continue",
    "fork",
    "join",
    "join_any",
    "join_none",
    "wait",
    "disable",
    "force",
    "release",
    "deassign",
    "defparam",
    "specify",
    "endspecify",
    "primitive",
    "endprimitive",
    "table",
    "endtable",
    "macromodule",
    "timescale",
    "include",
    "define",
    "ifdef",
    "ifndef",
    "elsif",
    "endif",
    "else",
    "undef",
];

const TYPES: &[&str] = &[
    "bit", "byte", "shortint", "int", "longint", "time", "realtime", "string", "void", "chandle",
    "event",
];

pub fn highlight(source: &str) -> Vec<Span> {
    let bytes = source.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0usize;
    let n = bytes.len();

    while i < n {
        // Line comment
        if i + 1 < n && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            let start = i;
            i += 2;
            while i < n && bytes[i] != b'\n' {
                i += 1;
            }
            spans.push(Span {
                start,
                end: i,
                kind: TokenKind::Comment,
            });
            continue;
        }

        // Block comment
        if i + 1 < n && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            let start = i;
            i += 2;
            while i + 1 < n && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < n {
                i += 2;
            }
            spans.push(Span {
                start,
                end: i,
                kind: TokenKind::Comment,
            });
            continue;
        }

        // String
        if bytes[i] == b'"' {
            let start = i;
            i += 1;
            while i < n {
                if bytes[i] == b'\\' && i + 1 < n {
                    i += 2;
                    continue;
                }
                if bytes[i] == b'"' {
                    i += 1;
                    break;
                }
                if bytes[i] == b'\n' {
                    break;
                }
                i += 1;
            }
            spans.push(Span {
                start,
                end: i,
                kind: TokenKind::String,
            });
            continue;
        }

        // Macro / compiler directive
        if bytes[i] == b'`' {
            let start = i;
            i += 1;
            while i < n && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            spans.push(Span {
                start,
                end: i,
                kind: TokenKind::Macro,
            });
            continue;
        }

        // Number (incl. sized literals like 8'hFF, 4'b1010)
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < n
                && (bytes[i].is_ascii_alphanumeric()
                    || matches!(bytes[i], b'\'' | b'_' | b'.' | b'x' | b'X' | b'z' | b'Z'))
            {
                i += 1;
            }
            spans.push(Span {
                start,
                end: i,
                kind: TokenKind::Number,
            });
            continue;
        }

        // Identifier / keyword
        if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' || bytes[i] == b'$' {
            let start = i;
            i += 1;
            while i < n && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'$')
            {
                i += 1;
            }
            let word = &source[start..i];
            let kind = if KEYWORDS.iter().any(|k| *k == word) {
                TokenKind::Keyword
            } else if TYPES.iter().any(|t| *t == word) || word.starts_with('$') {
                TokenKind::Type
            } else {
                TokenKind::Text
            };
            spans.push(Span { start, end: i, kind });
            continue;
        }

        // Single punctuation / whitespace
        let start = i;
        i += 1;
        spans.push(Span {
            start,
            end: i,
            kind: TokenKind::Text,
        });
    }

    spans
}
