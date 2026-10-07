# Appendix A: Chapter Maps to the Earlier Books

Two books precede this one, and either prepares you for it: [*The UVM Primer*](https://www.uvmprimer.com) (SystemVerilog) and [*Python for RTL Verification*](https://a.co/d/0hTKAJvh) (Python/cocotb/pyuvm). This appendix maps every chapter of this book to its companion chapters in both, for readers who want to compare treatments — or to lend the right book to a colleague. A dash means the topic has no mirror in that book; Chapters 5 and 21 cover ground (ownership; macros) neither predecessor made visible.

| Rust for RTL Verification | The UVM Primer | Python for RTL Verification |
|---|---|---|
| Ch. 1: Why Rust? | Ch. 1: Introduction | Why Python and why UVM? |
| Ch. 2: Rust Concepts | — | Python concepts |
| Ch. 3: Rust Basics | — | Python basics |
| Ch. 4: Conditions, Loops, and match | — | Conditions and loops / Ranges |
| Ch. 5: Ownership | *(no mirror — GC did this silently)* | *(no mirror)* |
| Ch. 6: Borrowing and References | *(no mirror)* | *(no mirror)* |
| Ch. 7: Structs, Enums, and Methods | Ch. 4: OOP; Ch. 7: Static Methods | Classes |
| Ch. 8: Collections | — | Python sequences / Lists / Strings / Dictionaries |
| Ch. 9: Result, Option, and the End of Exceptions | — | Exceptions |
| Ch. 10: Traits | Ch. 5: Classes and Extension; Ch. 6: Polymorphism | Inheritance / super() / protocols |
| Ch. 11: Generics | Ch. 8: Parameterized Class Definitions | (duck typing, throughout) |
| Ch. 12: Closures and Iterators | — | Generators / comprehensions |
| Ch. 13: Smart Pointers | — | Protecting attributes |
| Ch. 14: Modules, Crates, and Cargo | — | Modules |
| Interlude: The Complete TinyALU Testbench | *(the destination, previewed)* | *(testbench 8.0, previewed)* |
| Ch. 15: async/await and the Executor | *(the simulator's scheduler, opened up)* | Coroutines |
| Ch. 16: Tasks, Channels, and Sim-Aware Queues | Ch. 17: Interthread Communication | cocotb Queue |
| Ch. 17: Simulating with rustdv-sim | — | Simulating with cocotb |
| Ch. 18: Basic Testbench: 1.0 | Ch. 2: A Conventional Testbench | Basic testbench: 1.0 |
| Ch. 19: TinyAluBfm | Ch. 3: Interfaces and BFMs | TinyAluBfm |
| Ch. 20: Struct-Based Testbench: 2.0 | Ch. 10: An Object-Oriented Testbench | Class-based testbench: 2.0 |
| Ch. 21: Macros | *(the `` `uvm_*_utils `` macros, demystified)* | (decorators; design patterns) |
| Ch. 22: Why UVM? | Ch. 1: Introduction | Why UVM? |
| Ch. 23: uvm_test Testbench: 3.0 | Ch. 11: UVM Tests | uvm_test testbench: 3.0 |
| Ch. 24: Components | Ch. 12: UVM Components | uvm_component |
| Ch. 25: uvm_env Testbench: 4.0 | Ch. 13: UVM Environments | uvm_env testbench: 4.0 |
| Ch. 26: Logging | Ch. 19: UVM Reporting | Logging |
| Ch. 27: Configuration | *(uvm_config_db, in passing)* | ConfigDB() |
| Ch. 28: Configuration Debugging | — | Debugging the ConfigDB() |
| Ch. 29: The Factory | Ch. 9: The Factory Pattern | The UVM factory |
| Ch. 30: Variation-Point Testbench: 5.0 | — | UVM factory testbench: 5.0 |
| Ch. 31: Component Communications | Ch. 14: A New Paradigm; Ch. 18: Put and Get Ports | Component communications |
| Ch. 32: Analysis Ports | Ch. 15: Talking to Multiple Objects | Analysis ports |
| Ch. 33: Components in Testbench 6.0 | Ch. 16: Analysis Ports in a Testbench | Components in testbench 6.0 |
| Ch. 34: Connections in Testbench 6.0 | Ch. 18: Put and Get in Action; Ch. 22: UVM Agents | Connections in testbench 6.0 |
| Ch. 35: Transactions | Ch. 20: Deep Operations; Ch. 21: UVM Transactions | uvm_object in Python |
| Ch. 36: Sequence Testbench: 7.0 | Ch. 23: UVM Sequences | Sequence testbench: 7.0 |
| Ch. 37: Out-of-Order Transactions: Testbench 7.1 | Ch. 23: UVM Sequences | Fibonacci testbench: 7.1 / get_response() testbench: 7.2 |
| Ch. 38: Fibonacci Testbench: 7.2 | Ch. 23: UVM Sequences | Fibonacci testbench: 7.1 |
| Ch. 39: Virtual Sequence Testbench: 8.0 | Ch. 23: UVM Sequences | Virtual sequence testbench: 8.0 |
| Ch. 40: The Complete TinyALU Testbench | *(no mirror)* | *(no mirror)* |

This book has no closing what-comes-next chapter — the earlier books each wrote one, and their futures arrived on their own schedules. The book ends with the testbench.
