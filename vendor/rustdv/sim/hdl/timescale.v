// Compiled FIRST so the directive applies to all subsequent files
// (tinyalu.sv declares no timescale). The rustdv Clock and Timer need
// ns-scale precision; without this, Icarus defaults to 1s/1s.
`timescale 1ns/1ns
