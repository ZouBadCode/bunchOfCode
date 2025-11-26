#include <iostream>
#include <string>
#include <algorithm>

using namespace std;

void modifyString(string& s) {
  for (size_t i = 0; i < s.length(); ++i) {
    if (s[i] == 'r' || s[i] == 'a' || s[i] == 'y') {
      s[i] = 'R';
    }
  }
}

void modifyX(string& s) {
  for (size_t i = 0; i < s.length(); ++i) {
    if (isalpha(s[i])) {
      s[i] = 'X';
    }
  }
}

int main() {
  std::string s = "vector_and_arrays";
  std::string s_to_x = "vector_and_arrays";
  cout << "Original string: " << s << endl;

  modifyString(s);
  modifyX(s_to_x);
  cout << "Modified string: " << s << endl;
  cout << "X version: " << s_to_x << endl;
  return 0;
}