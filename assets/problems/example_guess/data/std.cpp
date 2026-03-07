#include <iostream>
#include <vector>

int get_num()
{
    std::cout << "get_num" << std::endl << std::flush;
    int ret;
    std::cin >> ret;
    return ret;
}

int guess(int index, int x)
{
    std::cout << "guess " << index << " " << x << std::endl << std::flush;
    int ret;
    std::cin >> ret;
    return ret;
}

void submit(const std::vector<int> &result)
{
    std::cout << "submit ";
    for (std::vector<int>::const_iterator iter = result.begin(); iter != result.end(); iter++)
    {
        std::cout << *iter << " ";
    }
    std::cout << std::endl << std::flush;
}


inline int solve(int i) {
	int l = 0, r = 1000000;
	while (l < r) {
		int mid = l + (r - l) / 2;
		if (guess(i, mid) >= 0) r = mid;
		else l = mid + 1;
	}
	return l;
}

int main() {
	int n = get_num();
	std::vector<int> a(n);
	for (int i = 0; i < n; i++) {
		a[i] = solve(i);
	}

	submit(a);
}
