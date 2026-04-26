#include "testlib.h"

namespace aij
{
    /// @brief 从流中取出并比较两个f64是否在允许误差范围内（包含绝对误差和相对误差）
    /// @param a
    /// @param b
    /// @param eps
    void doubleCompareComplex(InStream &a, InStream &b, double eps)
    {
        double ja = a.readDouble();
        double pa = b.readDouble();

        if (!::doubleCompare(ja, pa, eps))
            quitf(_wa, "expected %.10f, found %.10f", ja, pa);
    }

    /// @brief 从流中取出并比较两个f64序列是否在允许误差范围内（包含绝对误差和相对误差）
    /// @param ouf
    /// @param ans
    /// @param eps
    void doubleCompareComplexArray(InStream &ouf, InStream &ans, double eps)
    {
        while (!ans.seekEof())
        {
            doubleCompareComplex(ouf, ans, eps);
        }
    }

    /// @brief 从流中取出并比较两个f64是否在允许绝对误差范围内
    /// @param a
    /// @param b
    /// @param eps
    void doubleCompareAbs(InStream &a, InStream &b, double eps)
    {
        double ja = a.readDouble();
        double pa = b.readDouble();

        if (std::fabs(ja - pa) > eps + 1E-15)
            quitf(_wa, "expected %.10f, found %.10f", ja, pa);
    }

    /// @brief 从流中取出并比较两个f64序列是否在允许绝对误差范围内
    /// @param ouf
    /// @param ans
    /// @param eps
    void doubleCompareAbsArray(InStream &ouf, InStream &ans, double eps)
    {
        while (!ans.seekEof())
        {
            doubleCompareAbs(ouf, ans, eps);
        }
    }

    /// @brief 按行比较两个输入流的内容是否相同
    /// @param ouf
    /// @param ans
    void fcmp(InStream &ouf, InStream &ans)
    {
        std::string strAnswer;

        int n = 0;
        while (!ans.eof())
        {
            std::string j = ans.readString();

            if (j.empty() && ans.eof())
                break;

            strAnswer = j;
            std::string p = ouf.readString();

            n++;

            if (j != p)
                quitf(_wa, "%d%s lines differ - expected: '%s', found: '%s'", n, englishEnding(n).c_str(),
                      compress(j).c_str(), compress(p).c_str());
        }
    }

    /// @brief 从流中取出并比较两个大整数是否相等
    /// @param ouf
    /// @param ans
    void hcmp(InStream &ouf, InStream &ans)
    {
        pattern pnum("0|-?[1-9][0-9]*");
        auto isNumeric = [&](const std::string &p)
        {
            return pnum.matches(p);
        };
        std::string ja = ans.readWord();
        std::string pa = ouf.readWord();
        if (!isNumeric(ja))
            quitf(_fail, "%s is not a valid integer", compress(ja).c_str());

        if (!ans.seekEof())
            quitf(_fail, "expected exactly one token in the answer file");

        if (!isNumeric(pa))
            quitf(_pe, "%s is not a valid integer", compress(pa).c_str());

        if (ja != pa)
            quitf(_wa, "expected '%s', found '%s'", compress(ja).c_str(), compress(pa).c_str());
    }

    /// @brief 从流中取出并比较两个i32是否相等
    /// @param ouf
    /// @param ans
    void icmp(InStream &ouf, InStream &ans)
    {
        int ja = ans.readInt();
        int pa = ouf.readInt();

        if (ja != pa)
            quitf(_wa, "expected %d, found %d", ja, pa);
    }

    /// @brief 按行比较两个输入流的内容是否相同（将每行按空白符分割成若干词后再比较）
    /// @param ouf
    /// @param ans
    void lcmp(InStream &ouf, InStream &ans)
    {
        using std::string;
        using std::stringstream;
        using std::vector;
        auto compareWords = [](const string &a, const string &b) -> bool
        {
            vector<string> va, vb;
            stringstream sa;

            sa << a;
            string cur;
            while (sa >> cur)
                va.push_back(cur);

            stringstream sb;
            sb << b;
            while (sb >> cur)
                vb.push_back(cur);

            return (va == vb);
        };

        string strAnswer;

        int n = 0;
        while (!ans.eof())
        {
            string j = ans.readString();

            if (j.empty() && ans.eof())
                break;

            string p = ouf.readString();
            strAnswer = p;

            n++;

            if (!compareWords(j, p))
                quitf(_wa, "%d%s lines differ - expected: '%s', found: '%s'", n, englishEnding(n).c_str(),
                      compress(j).c_str(), compress(p).c_str());
        }
    }

    /// @brief 单行比较两个输入流中的i64序列是否相同
    /// @param ouf
    /// @param ans
    void ncmp(InStream &ouf, InStream &ans)
    {
        using std::vector;
        vector<long long> ansValues, oufValues;
        while (!ans.seekEoln())
            ansValues.push_back(ans.readLong());
        while (!ouf.seekEoln())
            oufValues.push_back(ouf.readLong());
        if (ansValues.size() != oufValues.size())
            quitf(_wa, "Expected %u elements, but %u found", (unsigned int)(ansValues.size()), (unsigned int)(oufValues.size()));
        for (size_t i = 0; i < ansValues.size(); i++)
        {
            if (ansValues[i] != oufValues[i])
                quitf(_wa, "%d%s numbers differ - expected: '%s', found: '%s'", (int)(i + 1), englishEnding((int)(i + 1)).c_str(),
                      vtos(ansValues[i]).c_str(), vtos(oufValues[i]).c_str());
        }
    }

    /// @brief 从流中取出并比较两个字符串是否同为"YES"或"NO"（不区分大小写）
    /// @param ouf
    /// @param ans
    void yesno(InStream &ouf, InStream &ans)
    {
        const std::string YES = "YES";
        const std::string NO = "NO";
        std::string ja = upperCase(ans.readWord());
        std::string pa = upperCase(ouf.readWord());

        if (ja != YES && ja != NO)
            quitf(_fail, "%s or %s expected in answer, but %s found", YES.c_str(), NO.c_str(), compress(ja).c_str());

        if (pa != YES && pa != NO)
            quitf(_pe, "%s or %s expected, but %s found", YES.c_str(), NO.c_str(), compress(pa).c_str());

        if (ja != pa)
            quitf(_wa, "expected %s, found %s", compress(ja).c_str(), compress(pa).c_str());
    }

    /// @brief 从流中取出一行并比较两个i64序列作为无序集合是否相等
    /// @param ouf
    /// @param ans
    void uncmp(InStream &ouf, InStream &ans)
    {
        using std::vector;
        vector<long long> ja, pa;
        while (!ans.seekEoln())
            ja.push_back(ans.readLong());
        while (!ouf.seekEoln())
            pa.push_back(ouf.readLong());
        if (ja.size() != pa.size())
            quitf(_wa, "Expected %u elements, but %u found", (unsigned int)(ja.size()), (unsigned int)(pa.size()));
        sort(ja.begin(), ja.end());
        sort(pa.begin(), pa.end());
        if (ja != pa)
            quitf(_wa, "Expected sequence and output are different (as unordered sequences) [size=%u]",
                  (unsigned int)(ja.size()));
    }

    /// @brief 消耗掉流中的所有内容
    /// @param stream
    void consumeStream(InStream &stream)
    {
        while (!stream.seekEof())
        {
            stream.readChar();
        }
    }

}