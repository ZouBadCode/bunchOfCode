#include<iostream>
using namespace std;

class Shape {
    public:
    double area() {
        return 0;
    }
};

class Rectangle : public Shape {
    private:
    double width;
    double height;
    public:
    Rectangle(double w, double h) : width(w), height(h) {}
    double area() {
        return width * height;
    }
};

class Circle : public Shape {
    private:
    double radius;
    public:
    Circle(double r) : radius(r) {}
    double area() {
        return 3.14159 * radius * radius;
    }
};

int main() {
    Shape* shape1 = new Rectangle(5.0, 3.0);
    Shape* shape2 = new Circle(4.0);

    cout << "Area of Rectangle: " << shape1->area() << endl;
    cout << "Area of Circle: " << shape2->area() << endl;

    delete shape1;
    delete shape2;

    return 0;
}