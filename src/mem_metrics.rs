/* Copyright 2021 Al Liu (https://github.com/al8n). Licensed under Apache-2.0.
 *
 * Copyright 2017 The Hashicorp's Raft repository authors(https://github.com/hashicorp/raft) authors. Licensed under MPL-2.0.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use metrics::{set_boxed_recorder, GaugeValue, Key, Recorder, Unit};
use parking_lot::{Once, RwLock};
use std::collections::HashMap;
use std::sync::Arc;

lazy_static! {
    static ref RECORDER: MemMetrics = MemMetrics::new();
    static ref SETUP: Once = Once::new();
}

/// MemMetrics implements the `metrics::Recorder` trait.
/// It should NOT EVER be used for production. It is used only for
/// unit tests.
#[derive(Default)]
pub struct MemMetrics {
    registered: Arc<RwLock<HashMap<Key, MetricsBasic>>>,
    gauge: Arc<RwLock<HashMap<Key, GaugeValue>>>,
    counter: Arc<RwLock<HashMap<Key, u64>>>,
    histogram: Arc<RwLock<HashMap<Key, f64>>>,
}

impl Clone for MemMetrics {
    fn clone(&self) -> Self {
        Self {
            registered: self.registered.clone(),
            gauge: self.gauge.clone(),
            counter: self.counter.clone(),
            histogram: self.histogram.clone(),
        }
    }
}

/// `MetricsType` represents the type of metric
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum MetricsType {
    Counter,
    Gauge,
    Histogram,
}

/// `MetricsBasic` stores the basic information for each metric
#[derive(Debug, Clone)]
pub struct MetricsBasic {
    typ: MetricsType,
    unit: Option<Unit>,
    description: Option<&'static str>,
}

impl Eq for MetricsBasic {}

impl PartialEq for MetricsBasic {
    fn eq(&self, other: &Self) -> bool {
        if self.typ != other.typ {
            return false;
        }

        return match self.clone().unit {
            None => other.unit == None,
            Some(unit) => match other.clone().unit {
                None => false,
                Some(ou) => ou == unit,
            },
        };
    }
}

impl MetricsBasic {
    pub fn new(typ: MetricsType, unit: Option<Unit>, description: Option<&'static str>) -> Self {
        Self {
            typ,
            unit,
            description,
        }
    }

    /// `from_type` returns a `MetricsBasic` according to `MetricsType`
    pub fn from_type(typ: MetricsType) -> Self {
        Self {
            typ,
            unit: None,
            description: None,
        }
    }

    /// `from_type_and_unit` returns a `MetricsBasic` according to `MetricsType` and `metrics::Unit`
    pub fn from_type_and_unit(typ: MetricsType, unit: Unit) -> Self {
        Self {
            typ,
            unit: Some(unit),
            description: None,
        }
    }
}

impl Recorder for MemMetrics {
    #[tracing::instrument(level = "trace", skip(self))]
    fn register_counter(&self, key: &Key, unit: Option<Unit>, description: Option<&'static str>) {
        self.registered.write().insert(
            key.clone(),
            MetricsBasic::new(MetricsType::Counter, unit, description),
        );
        self.counter.write().insert(key.clone(), 0);
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn register_gauge(&self, key: &Key, unit: Option<Unit>, description: Option<&'static str>) {
        self.registered.write().insert(
            key.clone(),
            MetricsBasic::new(MetricsType::Gauge, unit, description),
        );
        self.gauge
            .write()
            .insert(key.clone(), GaugeValue::Increment(0.0));
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn register_histogram(&self, key: &Key, unit: Option<Unit>, description: Option<&'static str>) {
        self.registered.write().insert(
            key.clone(),
            MetricsBasic::new(MetricsType::Histogram, unit, description),
        );
        self.histogram.write().insert(key.clone(), 0.0);
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn increment_counter(&self, key: &Key, value: u64) {
        let reg = &mut self.registered.write();

        match reg.get_mut(key) {
            None => {
                reg.insert(
                    key.clone(),
                    MetricsBasic {
                        typ: MetricsType::Counter,
                        unit: None,
                        description: None,
                    },
                );
                self.counter.write().insert(key.clone(), 0);
            }
            Some(_) => {
                *self.counter.write().get_mut(key).unwrap() += value;
            }
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn update_gauge(&self, key: &Key, value: GaugeValue) {
        let reg = &mut self.registered.write();

        match reg.get_mut(key) {
            None => {
                reg.insert(
                    key.clone(),
                    MetricsBasic {
                        typ: MetricsType::Gauge,
                        unit: None,
                        description: None,
                    },
                );
                self.gauge.write().insert(key.clone(), value);
            }
            Some(_) => {
                self.gauge.write().insert(key.clone(), value);
            }
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn record_histogram(&self, key: &Key, value: f64) {
        let reg = &mut self.registered.write();

        match reg.get_mut(key) {
            None => {
                reg.insert(
                    key.clone(),
                    MetricsBasic {
                        typ: MetricsType::Histogram,
                        unit: None,
                        description: None,
                    },
                );
                self.histogram.write().insert(key.clone(), value);
            }
            Some(_) => {
                self.histogram.write().insert(key.clone(), value);
            }
        }
    }
}

impl MemMetrics {
    pub fn new() -> Self {
        Self {
            registered: Arc::new(RwLock::new(HashMap::new())),
            gauge: Arc::new(RwLock::new(HashMap::new())),
            counter: Arc::new(RwLock::new(HashMap::new())),
            histogram: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn get_registered(&self, key: Key) -> Option<MetricsBasic> {
        match self.registered.read().get(&key) {
            None => None,
            Some(v) => Some(v.clone()),
        }
    }

    fn get_gauge(&self, key: Key) -> Option<GaugeValue> {
        match self.gauge.read().get(&key) {
            None => None,
            Some(v) => Some(v.clone()),
        }
    }

    fn get_counter(&self, key: Key) -> Option<u64> {
        match self.counter.read().get(&key) {
            None => None,
            Some(v) => Some(*v),
        }
    }

    fn get_histogram(&self, key: Key) -> Option<f64> {
        match self.histogram.read().get(&key) {
            None => None,
            Some(v) => Some(*v),
        }
    }
}

/// `setup_mem_metrics` will set a global in-memory metrics recorder
pub fn setup_mem_metrics() {
    SETUP.call_once(|| {
        set_boxed_recorder(box RECORDER.clone()).unwrap();
    });
}

/// `get_registered` returns the registered metric information according to the `metrics::Key`, if the key not exist, return `None`.
pub fn get_registered(key: Key) -> Option<MetricsBasic> {
    RECORDER.get_registered(key)
}

/// `get_gauge` returns a gauge metric according to the `metrics::Key`, if the key not exist, return `None`.
pub fn get_gauge(key: Key) -> Option<GaugeValue> {
    RECORDER.get_gauge(key)
}

/// `get_counter` returns a counter metric according to the `metrics::Key`, if the key not exist, return `None`.
pub fn get_counter(key: Key) -> Option<u64> {
    RECORDER.get_counter(key)
}

/// `get_histogram` returns a histogram metric according to the `metrics::Key`, if the key not exist, return `None`.
pub fn get_histogram(key: Key) -> Option<f64> {
    RECORDER.get_histogram(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use metrics::{
        gauge, histogram, increment_counter, register_counter, register_gauge, register_histogram,
        Key, Unit,
    };

    #[test]
    fn test_mem_metrics() {
        setup_mem_metrics();

        register_counter!(
            "requests_processed",
            Unit::Count,
            "number of requests processed"
        );
        increment_counter!("requests_processed");
        increment_counter!("requests_processed");

        let requests_processed_key = Key::from("requests_processed");

        let requests_processed = get_registered(requests_processed_key.clone()).unwrap();

        assert_eq!(
            requests_processed,
            MetricsBasic {
                typ: MetricsType::Counter,
                unit: Option::from(Unit::Count),
                description: Some("number of requests processed"),
            }
        );

        let requests_processed_ctr = get_counter(requests_processed_key).unwrap();
        assert_eq!(requests_processed_ctr, 2,);

        register_gauge!("gauge");
        gauge!("gauge", 9.0);

        let gauge_key = Key::from_static_name("gauge");

        let gi = get_registered(gauge_key.clone()).unwrap();
        assert_eq!(gi, MetricsBasic::from_type(MetricsType::Gauge));

        let g = get_gauge(gauge_key.clone()).unwrap();
        assert_eq!(format!("{:?}", g), "Absolute(9.0)");

        register_histogram!("unused_histogram", Unit::Seconds);
        histogram!("unused_histogram", 70.0);

        let histogram_key = Key::from_static_name("unused_histogram");

        let hi = get_registered(histogram_key.clone()).unwrap();
        assert_eq!(
            hi,
            MetricsBasic::from_type_and_unit(MetricsType::Histogram, Unit::Seconds)
        );

        let h = get_histogram(histogram_key).unwrap();
        assert_eq!(h, 70.0);
    }
}
