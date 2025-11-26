from abc import ABC, abstractmethod

class Machine(ABC):
    @abstractmethod
    def start(self) -> None: ...

class CoffeeMachine(Machine):
    def __init__(self):
        pass
    def start(self) -> None:
        print("Coffee started.")