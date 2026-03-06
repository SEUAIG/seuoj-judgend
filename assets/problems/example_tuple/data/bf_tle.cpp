#include<iostream>
using namespace std;

#include<vector>
int main() {
	int n; cin >> n;
	vector<long long>v1(n);
	for (long long i = 0; i < n; i++) {
		cin >> v1[i];
	}
	int num = 0;
	for (long long i = 0; i + 2 < n; i++) {
		for (long long j = i+1; j+1 < n; j++) {
			for (long long m = j+1; m  < n; m++) {
				if (v1[i] == v1[j] && v1[i] != v1[m]) {
					num++;
				}
				if (v1[i] == v1[m] && v1[i] != v1[j]) {
					num++;
				}
				if (v1[m] == v1[j] && v1[i] != v1[m]) {
					num++;
				}
			}
		}
	}
	cout << num;
	return 0;
}
