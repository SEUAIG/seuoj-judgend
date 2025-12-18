/// Supported programming languages for the judger system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SupportedLanguages {
    C,
    Cpp,
    Cpp11,
    Cpp17,
    Cpp20,
    Python3_12,
    Nodejs22,
    Go1_22,
    Java17,
}