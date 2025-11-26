#include <iostream>
#include <string>
using namespace std;

struct Student {
    string name;
    int age;
    double GPA;
};

int main() {
    Student student1;
    Student student2;

    cout << "student1 name" << endl;
    cin >> student1.name;
    cout << "student1 age" << endl;
    cin >> student1.age;
    cout << "student1 GPA" << endl;
    cin >> student1.GPA;

    cout << "student2 name" << endl;
    cin >> student1.name;
    cout << "student2 age" << endl;
    cin >> student1.age;    
    cout << "student2 GPA" << endl;
    cin >> student2.GPA;
    
    double AGPA = (student1.GPA + student2.GPA) / 2;
    cout << "average GPA" << AGPA << endl;

    cout << "student 1 info: " << endl << "    name: " << student1.name << "    age: " << student1.age << "    GPA: " << student1.GPA << endl;
    cout << "student 2 info: " << endl << "    name: " << student2.name << "    age: " << student2.age << "    GPA: " << student2.GPA << endl;

}