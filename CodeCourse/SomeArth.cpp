#include <iostream>
using namespace std;

int main() {
    int a[8] = { 1,2,3,4,5,6,7,8 };
    int* p = a;

    int sum = 0;
    for (int* ptr = a; ptr < a + 8; ++ptr)
        sum += *ptr;
    cout << "Sum = " << sum << endl;

    for (int *left = a, *right = a + 7; left < right; left++, --right) {
        int tmp = *left;
        *left = *right;
        *right = tmp;
    }

    for (int* ptr = a + 2; ptr < a + 6; ++ptr)
        *ptr = 0;

    cout << "Array = ";
    for (int* ptr = a; ptr < a + 8; ++ptr)
        cout << *ptr << " ";
    cout << endl;

    return 0;
}