# Rust for RTL Verification — Chapter 13, Figure 2
# "The refcount Python never showed you"
# Python contrast figure. Run: python3 fig02_refcount_python_never_showed.py


import sys

a = ["ADD", 5, 3]
print(sys.getrefcount(a))
b = a
print(sys.getrefcount(a))
