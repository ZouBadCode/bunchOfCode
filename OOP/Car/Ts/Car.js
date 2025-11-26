var Car = /** @class */ (function () {
    function Car(engineId) {
        this.engineId = engineId;
    }
    Car.prototype.start = function () { console.log("Car ".concat(this.engineId, " starts!")); };
    return Car;
}());
var v = new Car("ENG-001");
v.start();
