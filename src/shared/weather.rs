use rand_distr::{Distribution, Normal};

use chrono::{Utc, DateTime};

use serde::{Serialize};

#[derive(Serialize)]
pub struct WeatherRecord {
    station_id: u8,
    temperature: f32,
    #[serde(with = "chrono::serde::ts_microseconds")]
    pub ts: DateTime<Utc>
}

pub struct WeatherStation {
    pub id: u8,
    pub name: String,
    avg_temp: f32,
}

impl WeatherStation {
    pub fn new(id: u8, name: &str, avg_temp: f32) -> Self {
        Self {
            id, name: name.to_string(), avg_temp
        }
    }

    pub fn generate_data(&self) -> WeatherRecord {
        let mut rng = rand::rng();
        let normal_dist_temp = Normal::new(self.avg_temp, 3.0).unwrap();
        let random_temp = normal_dist_temp.sample(&mut rng);
        

        let now: DateTime<Utc> = Utc::now();

        WeatherRecord {
            station_id: self.id,
            temperature: random_temp,
            ts: now
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
