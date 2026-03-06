#include <iostream>
#include <vector>
using std::cin, std::cout;
using std::vector;
int main(){
    int n; cin >> n;
    vector<int>cnt(n+1);
    for (int i = 1; i <= n; ++i){
        int x; cin >> x;
        ++cnt[x];
    }
    long long ans = 0;
    for (int i = 1; i <= n; ++i)if(cnt[i] >= 2){
        int t = n - cnt[i];
        ans += 1ll * t * cnt[i] * (cnt[i] - 1) >> 1;
    }
    cout << ans << '\n';
    return 0;
}