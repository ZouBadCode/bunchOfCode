from abc import ABC, abstractmethod

class Vehicle(ABC):
    @abstractmethod
    def start(self): ...

class Car(Vehicle):
    def __init__(self, engine_id):
        self.engine_id = engine_id
    def start(self):
        print(f"Car {self.engine_id} starts!")

v = Car("eng-001")
v.start()        