package OOP.Car.Java;
interface Vehicle { void start(); }

class Car implements Vehicle {
    private String engineId;
    public Car(String engineId) {
        this.engineId = engineId;
    }
    public void start() { System.out.println("Car " + engineId + " Start!"); }
}

class Demo {
    public static void main(String[] args) {
        Vehicle v = new Car("V8");
        v.start();
    }
}