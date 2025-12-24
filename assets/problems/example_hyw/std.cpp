#include <iostream>
#include <string>
using std::cin, std::cout;
using std::string;
int main() {
    string s; cin >> s;
    int n = s.length();
    for (int i = 2; i < n; ++i)
        for (int j = i; j >= 2; j -= 2) 
            if (s[j - 2] == 'h' && s[j - 1] == 'y' && s[j] == 'w') {
                s[j - 2] = 'w';
                s[j - 1] = 'z';
                s[j] = 'h';
            }else break;
    cout << s << '\n';
    return 0;
}
