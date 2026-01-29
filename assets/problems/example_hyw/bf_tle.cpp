#include <cstring>
#include <iostream>

int findsubstring(char* str, int length, std::string sub) {
    for (int i = 0; i < length - 2; i++) {
        if (str[i] == 'h' && str[i + 1] == 'y' && str[i + 2] == 'w') {
            return i;
        }
    }
    return -1;
}
void replace(char* str, int index) {
    str[index] = 'w';
    str[index + 1] = 'z';
    str[index + 2] = 'h';
}

int main() {
    char str[200000];
    memset(str, 0, sizeof(str));
    std::cin >> str;
    int i;
    while ((i = findsubstring(str, strlen(str), "hyw")) != -1) {
        replace(str, i);
    }
    std::cout << str << std::endl;
    return 0;
}
