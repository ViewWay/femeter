/* ================================================================== */
/*                                                                    */
/*  load_forecast.rs — 边缘计算: 本地负荷预测                           */
/*                                                                    */
/*  滑动窗口线性回归、EWMA 短期预测、负荷模式识别、                     */
/*  MAPE/RMSE 精度评估。嵌入式友好, 内存 <2KB。                         */
/*                                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

/// 滑动窗口最大深度
pub const WINDOW_SIZE: usize = 48;
/// EWMA 平滑系数 (0~1, 越大越敏感)
pub const EWMA_ALPHA: f32 = 0.3;

/// 负荷模式
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum LoadPattern {
    #[default]
    Unknown = 0,
    Weekday = 1,
    Weekend = 2,
    Summer = 3,
    Winter = 4,
    SpringAutumn = 5,
    Peak = 6,
    Valley = 7,
    Flat = 8,
}

/// 时间段信息 (用于模式识别)
#[derive(Clone, Copy, Debug, Default)]
pub struct TimeContext {
    pub hour: u8,
    pub day_of_week: u8, // 0=Mon..6=Sun
    pub month: u8,       // 1..12
    pub is_holiday: bool,
}

impl TimeContext {
    pub fn new(hour: u8, day_of_week: u8, month: u8) -> Self {
        Self {
            hour,
            day_of_week,
            month,
            is_holiday: false,
        }
    }

    /// 识别负荷模式
    pub fn pattern(&self) -> LoadPattern {
        let is_weekend = self.day_of_week >= 5;

        // 时段判断
        let is_peak = self.hour >= 9 && self.hour <= 11 || self.hour >= 17 && self.hour <= 21;
        let is_valley = self.hour >= 23 || self.hour <= 6;

        if is_peak {
            LoadPattern::Peak
        } else if is_valley {
            LoadPattern::Valley
        } else if is_weekend {
            LoadPattern::Weekend
        } else {
            LoadPattern::Weekday
        }
    }
}

// ── 滑动窗口线性回归预测 ──

/// 线性回归结果
#[derive(Clone, Copy, Debug, Default)]
pub struct LinearRegression {
    pub slope: f32,
    pub intercept: f32,
}

/// 滑动窗口 + 线性回归预测器
#[derive(Clone, Debug)]
pub struct LinearForecast {
    /// 数据窗口
    pub window: [f32; WINDOW_SIZE],
    /// 当前窗口深度
    pub depth: usize,
    /// 回归结果
    pub regression: LinearRegression,
}

impl LinearForecast {
    pub fn new() -> Self {
        Self {
            window: [0.0; WINDOW_SIZE],
            depth: 0,
            regression: LinearRegression::default(),
        }
    }

    /// 添加新数据点
    pub fn push(&mut self, value: f32) {
        if self.depth < WINDOW_SIZE {
            self.window[self.depth] = value;
            self.depth += 1;
        } else {
            // 滑动: 移除最老的
            self.window.copy_within(1.., 0);
            self.window[WINDOW_SIZE - 1] = value;
        }
    }

    /// 计算线性回归 y = slope * x + intercept
    pub fn fit(&mut self) {
        let n = self.depth;
        if n < 2 {
            self.regression = LinearRegression::default();
            return;
        }
        let sum_x = (n * (n - 1)) as f32 / 2.0;
        let sum_y: f32 = self.window[..n].iter().sum();
        let sum_xy: f32 = self.window[..n]
            .iter()
            .enumerate()
            .map(|(i, y)| i as f32 * y)
            .sum();
        let sum_x2: f32 = (0..n).map(|i| (i * i) as f32).sum();

        let denom = n as f32 * sum_x2 - sum_x * sum_x;
        if denom.abs() < 1e-10 {
            self.regression = LinearRegression::default();
            return;
        }

        self.regression.slope = (n as f32 * sum_xy - sum_x * sum_y) / denom;
        self.regression.intercept = (sum_y - self.regression.slope * sum_x) / n as f32;
    }

    /// 预测下一步
    pub fn predict_next(&self) -> f32 {
        if self.depth < 2 {
            return self.window[0];
        }
        let x = self.depth as f32;
        self.regression.slope * x + self.regression.intercept
    }

    /// 预测 N 步
    pub fn predict(&self, steps: usize) -> f32 {
        if self.depth < 2 {
            return self.window[0];
        }
        let x = (self.depth + steps) as f32;
        self.regression.slope * x + self.regression.intercept
    }

    /// 获取窗口均值
    pub fn mean(&self) -> f32 {
        if self.depth == 0 {
            return 0.0;
        }
        self.window[..self.depth].iter().sum::<f32>() / self.depth as f32
    }
}

impl Default for LinearForecast {
    fn default() -> Self {
        Self::new()
    }
}

// ── EWMA 指数加权移动平均 ──

/// EWMA 预测器
#[derive(Clone, Copy, Debug)]
pub struct EwmaForecast {
    /// 平滑值
    pub smoothed: f32,
    /// 平滑系数
    pub alpha: f32,
    /// 初始化标志
    pub initialized: bool,
}

impl EwmaForecast {
    pub fn new(alpha: f32) -> Self {
        Self {
            smoothed: 0.0,
            alpha,
            initialized: false,
        }
    }

    /// 更新 EWMA
    pub fn update(&mut self, value: f32) {
        if !self.initialized {
            self.smoothed = value;
            self.initialized = true;
        } else {
            self.smoothed = self.alpha * value + (1.0 - self.alpha) * self.smoothed;
        }
    }

    /// 预测下一步 (EWMA 预测值即为当前平滑值)
    pub fn predict(&self) -> f32 {
        self.smoothed
    }

    /// 重置
    pub fn reset(&mut self) {
        self.smoothed = 0.0;
        self.initialized = false;
    }
}

impl Default for EwmaForecast {
    fn default() -> Self {
        Self::new(EWMA_ALPHA)
    }
}

// ── 精度评估 ──

/// 预测精度指标
#[derive(Clone, Copy, Debug, Default)]
pub struct ForecastAccuracy {
    /// MAPE (平均绝对百分比误差, %)
    pub mape: f32,
    /// RMSE (均方根误差)
    pub rmse: f32,
    /// 最大绝对误差
    pub max_error: f32,
    /// 样本数
    pub sample_count: u32,
}

/// 计算预测精度
pub fn evaluate_forecast(actual: &[f32], predicted: &[f32]) -> ForecastAccuracy {
    let n = actual.len().min(predicted.len());
    if n == 0 {
        return ForecastAccuracy::default();
    }

    let mut sum_abs_pct_err = 0.0f32;
    let mut sum_sq_err = 0.0f32;
    let mut max_err = 0.0f32;
    let mut valid_count = 0u32;

    for i in 0..n {
        let err = (actual[i] - predicted[i]).abs();
        sum_sq_err += err * err;
        if err > max_err {
            max_err = err;
        }
        if actual[i].abs() > 1e-10 {
            sum_abs_pct_err += err / actual[i].abs();
            valid_count += 1;
        }
    }

    let mape = if valid_count > 0 {
        (sum_abs_pct_err / valid_count as f32) * 100.0
    } else {
        0.0
    };
    let rmse = (sum_sq_err / n as f32).sqrt();

    ForecastAccuracy {
        mape,
        rmse,
        max_error: max_err,
        sample_count: n as u32,
    }
}

// ── 综合负荷预测器 ──

/// 综合预测结果
#[derive(Clone, Copy, Debug, Default)]
pub struct LoadForecastResult {
    /// 线性回归预测值
    pub linear_value: f32,
    /// EWMA 预测值
    pub ewma_value: f32,
    /// 综合预测 (加权平均)
    pub combined: f32,
    /// 当前负荷模式
    pub pattern: LoadPattern,
    /// 置信度 (0~1, 基于数据量)
    pub confidence: f32,
}

/// 综合负荷预测器 (嵌入式友好, <2KB)
#[derive(Clone, Debug)]
pub struct LoadForecaster {
    pub linear: LinearForecast,
    pub ewma: EwmaForecast,
}

impl LoadForecaster {
    pub fn new() -> Self {
        Self {
            linear: LinearForecast::new(),
            ewma: EwmaForecast::new(EWMA_ALPHA),
        }
    }

    /// 更新并预测
    pub fn update(&mut self, value: f32) -> LoadForecastResult {
        self.linear.push(value);
        self.linear.fit();
        self.ewma.update(value);

        let linear_val = self.linear.predict_next();
        let ewma_val = self.ewma.predict();

        // 加权: 数据少时偏 EWMA, 数据多时偏线性回归
        let w = if self.linear.depth < 10 {
            0.2
        } else if self.linear.depth < 30 {
            0.5
        } else {
            0.7
        };

        LoadForecastResult {
            linear_value: linear_val,
            ewma_value: ewma_val,
            combined: w * linear_val + (1.0 - w) * ewma_val,
            pattern: LoadPattern::Unknown,
            confidence: (self.linear.depth as f32 / WINDOW_SIZE as f32).min(1.0),
        }
    }

    /// 带时间上下文的更新
    pub fn update_with_context(&mut self, value: f32, ctx: &TimeContext) -> LoadForecastResult {
        let mut result = self.update(value);
        result.pattern = ctx.pattern();
        result
    }

    /// 重置
    pub fn reset(&mut self) {
        self.linear = LinearForecast::new();
        self.ewma.reset();
    }
}

impl Default for LoadForecaster {
    fn default() -> Self {
        Self::new()
    }
}

// ── 内存占用检查 ──

#[allow(dead_code)]
const _: () = {
    // LoadForecaster: LinearForecast(48*4 + 4 + 8) + EwmaForecast(12) = ~220 bytes
    assert!(core::mem::size_of::<LoadForecaster>() < 2048);
    assert!(core::mem::size_of::<LoadForecastResult>() < 32);
};

// ══════════════════════════════════════════════════════════════════
//  单元测试
// ══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_context_pattern_weekday_peak() {
        let ctx = TimeContext::new(10, 0, 3); // Wed 10am Mar
        assert_eq!(ctx.pattern(), LoadPattern::Peak);
    }

    #[test]
    fn test_time_context_pattern_valley() {
        let ctx = TimeContext::new(3, 0, 3);
        assert_eq!(ctx.pattern(), LoadPattern::Valley);
    }

    #[test]
    fn test_time_context_pattern_summer() {
        let ctx = TimeContext::new(14, 0, 7); // Mon 2pm Jul
                                              // 14:00 is not peak/valley, not weekend → Weekday (season patterns removed from priority)
        assert_eq!(ctx.pattern(), LoadPattern::Weekday);
    }

    #[test]
    fn test_time_context_pattern_winter() {
        let ctx = TimeContext::new(14, 0, 1); // Mon 2pm Jan
        assert_eq!(ctx.pattern(), LoadPattern::Weekday);
    }

    #[test]
    fn test_time_context_pattern_weekend() {
        let ctx = TimeContext::new(14, 5, 4); // Sat 2pm Apr
        assert_eq!(ctx.pattern(), LoadPattern::Weekend);
    }

    #[test]
    fn test_linear_forecast_empty() {
        let lf = LinearForecast::new();
        assert!((lf.mean() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_linear_forecast_single_point() {
        let mut lf = LinearForecast::new();
        lf.push(100.0);
        assert!((lf.mean() - 100.0).abs() < 1e-6);
        assert!((lf.predict_next() - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_linear_forecast_trend() {
        let mut lf = LinearForecast::new();
        // 线性增长: 100, 102, 104, ...
        for i in 0..20 {
            lf.push(100.0 + i as f32 * 2.0);
        }
        lf.fit();
        assert!(
            lf.regression.slope > 1.9,
            "slope too low: {}",
            lf.regression.slope
        );
        assert!(
            lf.regression.slope < 2.1,
            "slope too high: {}",
            lf.regression.slope
        );
    }

    #[test]
    fn test_linear_forecast_sliding_window() {
        let mut lf = LinearForecast::new();
        for _i in 0..WINDOW_SIZE + 10 {
            lf.push(100.0);
        }
        assert_eq!(lf.depth, WINDOW_SIZE);
        assert!((lf.mean() - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_linear_forecast_predict() {
        let mut lf = LinearForecast::new();
        for i in 0..10 {
            lf.push(10.0 + i as f32);
        }
        lf.fit();
        let pred = lf.predict(5);
        assert!(pred > 14.0, "prediction too low: {}", pred);
    }

    #[test]
    fn test_ewma_basic() {
        let mut ewma = EwmaForecast::new(0.5);
        ewma.update(100.0);
        assert!((ewma.predict() - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_ewma_smoothing() {
        let mut ewma = EwmaForecast::new(0.3);
        ewma.update(100.0);
        ewma.update(200.0);
        let val = ewma.predict();
        // alpha*200 + (1-alpha)*100 = 60 + 70 = 130
        assert!((val - 130.0).abs() < 0.1, "EWMA value: {}", val);
    }

    #[test]
    fn test_ewma_reset() {
        let mut ewma = EwmaForecast::new(0.5);
        ewma.update(100.0);
        ewma.reset();
        assert!(!ewma.initialized);
        assert!((ewma.predict() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_evaluate_forecast_perfect() {
        let actual = [100.0, 200.0, 300.0];
        let predicted = [100.0, 200.0, 300.0];
        let acc = evaluate_forecast(&actual, &predicted);
        assert!((acc.mape - 0.0).abs() < 1e-6);
        assert!((acc.rmse - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_evaluate_forecast_errors() {
        let actual = [100.0, 200.0, 300.0];
        let predicted = [110.0, 190.0, 310.0];
        let acc = evaluate_forecast(&actual, &predicted);
        assert!(acc.mape > 0.0);
        assert!(acc.rmse > 0.0);
        assert!(acc.max_error >= 10.0);
        assert_eq!(acc.sample_count, 3);
    }

    #[test]
    fn test_evaluate_forecast_empty() {
        let acc = evaluate_forecast(&[], &[]);
        assert!((acc.mape - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_evaluate_forecast_uneven() {
        let actual = [100.0, 200.0];
        let predicted = [110.0, 190.0, 300.0]; // longer predicted
        let acc = evaluate_forecast(&actual, &predicted);
        assert_eq!(acc.sample_count, 2);
    }

    #[test]
    fn test_load_forecaster_basic() {
        let mut fc = LoadForecaster::new();
        for i in 0..20 {
            let r = fc.update(100.0 + i as f32);
            assert!(r.confidence > 0.0);
        }
    }

    #[test]
    fn test_load_forecaster_with_context() {
        let mut fc = LoadForecaster::new();
        let ctx = TimeContext::new(10, 2, 7); // Wed 10am Jul
        let r = fc.update_with_context(500.0, &ctx);
        assert_eq!(r.pattern, LoadPattern::Peak);
    }

    #[test]
    fn test_load_forecaster_confidence_growth() {
        let mut fc = LoadForecaster::new();
        let c0 = fc.update(100.0).confidence;
        for _i in 0..WINDOW_SIZE {
            fc.update(100.0);
        }
        let c_full = fc.update(100.0).confidence;
        assert!(c_full > c0);
        assert!((c_full - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_load_forecaster_reset() {
        let mut fc = LoadForecaster::new();
        fc.update(100.0);
        fc.reset();
        assert_eq!(fc.linear.depth, 0);
        assert!(!fc.ewma.initialized);
    }

    #[test]
    fn test_memory_size_load_forecaster() {
        assert!(
            core::mem::size_of::<LoadForecaster>() < 2048,
            "LoadForecaster too large: {} bytes",
            core::mem::size_of::<LoadForecaster>()
        );
    }
}
