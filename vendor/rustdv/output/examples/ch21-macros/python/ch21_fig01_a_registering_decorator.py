# Chapter 21, Figure 1: What @cocotb.test() really does — a registering decorator
# Run: python3 ch21-macros/python/ch21_fig01_a_registering_decorator.py

test_registry = []

def test():
    def wrapper(coro):
        test_registry.append(coro)   # side effect at import time
        return coro
    return wrapper

@test()
def hello_world():
    print("Hello, world.")

@test()
def wait_2ns():
    print("I am DONE waiting!")

print(f"registered {len(test_registry)} tests:")
for t in test_registry:
    print(f"  {t.__name__}")
