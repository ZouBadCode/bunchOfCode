#include <iostream>
#include <string>
#include "comparison.h"
using namespace std;

int main() {
    int input;
    int input2;
    signed int result;
    string results;
    cout << "input 1: " << endl;
    cin >> input;
    cout << "input 2:" << endl;
    cin >> input2;

    if (input > input2) {
        result = -1;
    } else if (input < input2) {
        result = 0;
    } else {
        result = 1;
    }

    if (result = -1) {
        results = "input1 is greater than input2";
    } else if(result = 0) {
        results = "input1 is less than input2";
    } else {
        results = "equal";
    }
    cout << results << endl;   
}