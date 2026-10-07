# Rust for RTL Verification — Chapter 5, Figure 1
# "Python assignment: two names, one object"
# Python contrast figure. Run: python3 fig01_python_assignment_two_names.py


a = ["ADD", 5, 3]
b = a
b[0] = "MUL"
print(a)
print(a is b)
