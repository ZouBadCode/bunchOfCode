#include <iostream>

using namespace std;

int main() {
  double arr[2][4] = {{3.5, -1.0, 0.0, 2.25}, {10.5, 7.0, -8.0, -4.5}};

  cout << "Array elements:" << endl;
  for (int i = 0; i < 2; ++i) {
    for (int j = 0; j < 4; ++j) {
      cout << arr[i][j] << " ";
    }
    cout << endl;
  }

  double rowSums[2];
  for (int i = 0; i < 2; ++i) {
    double sum = 0.0;
    for (int j = 0; j < 4; ++j) {
      sum += arr[i][j];
    }
    rowSums[i] = sum;
  }

  double grandTotal = 0.0;
  for (int i = 0; i < 2; ++i) {
    grandTotal += rowSums[i];
  }

  cout << "\nRow sums:" << endl;
  for (int i = 0; i < 2; ++i) {
    cout << "Row " << i + 1 << ": " << rowSums[i] << endl;
  }

  cout << "\nGrand total: " << grandTotal << endl;

  return 0;
}