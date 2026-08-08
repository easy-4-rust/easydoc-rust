/// 转换过程中发生的可恢复降级说明。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversionWarning {
    /// 面向调用方的降级信息。
    pub message: String,
}
