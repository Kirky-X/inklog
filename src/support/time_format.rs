// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! chrono `to_rfc3339()` 的等价格式钉：UTC 用 `+00:00`，小数秒按 0/3/6/9 位分组。

use time::OffsetDateTime;
use time::format_description::StaticFormatDescription;
use time::macros::format_description;

const RFC3339_NO_FRAC: StaticFormatDescription = format_description!(
    "[year]-[month]-[day]T[hour]:[minute]:[second][offset_hour sign:mandatory]:[offset_minute]"
);
const RFC3339_MILLIS: StaticFormatDescription = format_description!(
    "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3][offset_hour sign:mandatory]:[offset_minute]"
);
const RFC3339_MICROS: StaticFormatDescription = format_description!(
    "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:6][offset_hour sign:mandatory]:[offset_minute]"
);
const RFC3339_NANOS: StaticFormatDescription = format_description!(
    "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:9][offset_hour sign:mandatory]:[offset_minute]"
);

/// 以 chrono `DateTime::to_rfc3339()` 的输出格式渲染 `OffsetDateTime`。
pub(crate) fn to_rfc3339(datetime: &OffsetDateTime) -> String {
    let nanos = datetime.nanosecond();
    let format = if nanos == 0 {
        RFC3339_NO_FRAC
    } else if nanos.is_multiple_of(1_000_000) {
        RFC3339_MILLIS
    } else if nanos.is_multiple_of(1_000) {
        RFC3339_MICROS
    } else {
        RFC3339_NANOS
    };
    datetime.format(format).expect("RFC3339 format")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339_pins_chrono_offset_and_precision() {
        let dt = OffsetDateTime::from_unix_timestamp(1_767_225_600).unwrap();
        assert_eq!(to_rfc3339(&dt), "2026-01-01T00:00:00+00:00");
        let millis = dt + time::Duration::milliseconds(123);
        assert_eq!(to_rfc3339(&millis), "2026-01-01T00:00:00.123+00:00");
        let micros = dt + time::Duration::microseconds(123_456);
        assert_eq!(to_rfc3339(&micros), "2026-01-01T00:00:00.123456+00:00");
        let nanos = dt + time::Duration::nanoseconds(123_456_789);
        assert_eq!(to_rfc3339(&nanos), "2026-01-01T00:00:00.123456789+00:00");
    }
}
