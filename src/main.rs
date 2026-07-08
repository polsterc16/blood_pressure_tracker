#![allow(unused)]
#![allow(unused_labels)]
use chrono::DateTime;
use chrono::Datelike;
use chrono::Local;
use chrono::NaiveDateTime;
use chrono::TimeDelta;
use chrono::Timelike;
use chrono::Utc;
use clap::Parser;
use pretty_simple_display::DebugPretty;
use serde::Deserialize;
use serde::Serialize;
use std::cmp::max;
use std::cmp::min;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::path::PathBuf;

// ################################################################

const CSV_HEADER: &str = "date,time,sys,dia,pul";
const SECS_IN_DAYS_F32: f32 = 86400_f32;

// ################################################################

/// Array describing a blood pressure measurement sample,
/// consisting of [[`dia`, `sys`, `pul`]].
type BpSampleType = [f32; 3];
/// `HashMap` that stores `CollectionDay` objs by their field `sec: i64`.
type CollDayHashType = HashMap<i64, CollectionDay>;

/// Vector of measurement samples (`f32`)
type VecMeasType = Vec<f32>;
/// Vector of measurement vectors ([`VecMeasType`])
type VecMeas2dType = Vec<VecMeasType>;

// ################################################################

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Add blood pressure measurement
    #[arg(short, long, num_args = 3, value_names=["sys","dia","pul"])]
    add: Option<Vec<f32>>,

    /// Generate Output
    #[arg(short, long)]
    output: bool,

    /// Display status of CSV file
    #[arg(short, long)]
    status: bool,

    /// Rebuild CSV file
    #[arg(short, long)]
    rebuild: bool,
}

// ################################################################

/// Sequence (`VecMeas2dType:Vec<VecMeasType>`) of measurements (`VecMeasType:Vec<f32>`).
#[derive(Debug, Serialize, Deserialize)]
struct F32Vec2d(VecMeas2dType);
impl F32Vec2d {
    /// Create new sequence (`VecMeas2dType`).
    fn new() -> Self {
        Self(Vec::<VecMeasType>::new())
    }
    /// Create new sequence (`VecMeas2dType`) for `size_x` measurements (`VecMeasType`).
    fn new_x(size_x: usize) -> Self {
        Self(Vec::<VecMeasType>::with_capacity(size_x))
    }
    /// Create new sequence (`VecMeas2dType`) for `size_x` measurements (`VecMeasType`),
    /// which each have reserved capacity `len_y`.
    fn new_xy(size_x: usize, len_y: usize) -> Self {
        // create sequence (`VecMeas2dType`) for `size_x` measurements
        let mut ret_item = Self::new_x(size_x);
        let mut ref_seq_vec = ret_item.get_ref_seq_vec_mut();

        // add `size_x` measurement `vec`s (`VecMeasType`) with capacity `len_y`
        for _ in 0..size_x {
            ref_seq_vec.push(VecMeasType::with_capacity(len_y));
        }
        return ret_item;
    }
    /// Get mut ref to internal sequence of measurements (`VecMeas2dType`)
    fn get_ref_seq_vec_mut(&mut self) -> &mut VecMeas2dType {
        &mut self.0
    }
    /// Get ref to internal sequence of measurements (`VecMeas2dType`)
    fn get_ref_seq_vec(&self) -> &VecMeas2dType {
        &self.0
    }
    /// Get mut ref to internal measurement (`VecMeasType`) of sequence at index `idx_m`
    /// # Panic
    /// Panics if `idx_m` not is in range of valid indexes.
    fn get_ref_meas_vec_mut(&mut self, idx_m: usize) -> &mut VecMeasType {
        self.check_meas_idx_range_panic(idx_m);
        &mut self.0[idx_m]
    }
    /// Get ref to internal measurement (`VecMeasType`) of sequence at index `idx_m`
    /// # Panic
    /// Panics if `idx_m` not is in range of valid indexes.
    fn get_ref_meas_vec(&self, idx_m: usize) -> &VecMeasType {
        self.check_meas_idx_range_panic(idx_m);
        &self.0[idx_m]
    }
    /// Returns *number* of measurement vectors in sequence.
    #[inline]
    fn get_meas_num(&self) -> usize {
        self.get_ref_seq_vec().len()
    }
    /// Returns *length* of measurement vectors in sequence.
    #[inline]
    fn get_meas_vec_len(&self) -> usize {
        self.get_ref_meas_vec(0).len()
    }
    /// Check if `idx_m` is in range of valid indexes for measurements in the sequence.
    /// ***
    /// See also [`check_meas_idx_range_panic`]
    fn check_meas_idx_range(&self, idx_m: usize) -> bool {
        let r = 0..self.get_meas_num();
        return r.contains(&idx_m);
    }
    /// Check if `idx_m` is in range of valid indexes for measurements in the sequence.
    /// # Panic
    /// Panics if `idx_m` not is in range of valid indexes.
    /// ***
    /// See also [`check_meas_idx_range`]
    fn check_meas_idx_range_panic(&self, idx_m: usize) {
        let r = 0..self.get_meas_num();

        if !r.contains(&idx_m) {
            panic!("Out-of-bounds index '{idx_m}' ({r:?})!")
        }
    }
    /// 1. Performs check if all measurement `vec`s are of the same length.
    /// 2. Tries to shrink the capacity of all measurement `vec`s.
    /// # Panic
    /// Panics if measurement `vec`s are not of the same length.
    fn validate_shrink(&mut self) {
        let (_, len) = self.get_dim_filled();
        if len.is_empty() {
            panic!("range is_empty ({len:?})!");
        }
        if len.start() != len.end() {
            panic!("`F32Vec2d` obj is not equally filled ({len:?})!");
        }
        // apply `shrink_to_fit` to all measurement `vec`s
        for vec_m in self.get_ref_seq_vec_mut() {
            vec_m.shrink_to_fit();
        }
    }
    /// Sorts all measurement `vec`s individually.
    fn sort_seq(&mut self) {
        for idx_m in 0..self.get_meas_num() {
            self.get_ref_meas_vec_mut(idx_m).sort_by(f32::total_cmp);
        }
    }
    /// Get the vector capacity in both dimensions:
    /// 1. Capacity of the sequence (`VecMeas2dType:Vec<VecMeasType>`)
    /// 2. Capacity range (`min..=max`) of the measurements (`VecMeasType:Vec<f32>`)
    fn get_dim_capacity(&self) -> (usize, std::ops::RangeInclusive<usize>) {
        let seq_vec = self.get_ref_seq_vec();
        let _min = seq_vec
            .iter()
            .fold(usize::MAX, |_min, v_f32| min(_min, v_f32.capacity()));
        let _max = seq_vec
            .iter()
            .fold(usize::MIN, |_max, v_f32| max(_max, v_f32.capacity()));
        (seq_vec.capacity(), _min..=_max)
    }
    /// Get the vector length in both dimensions:
    /// 1. Length of the sequence (`VecMeas2dType:Vec<VecMeasType>`)
    /// 2. Length range (`min..=max`) of the measurements (`VecMeasType:Vec<f32>`)
    fn get_dim_filled(&self) -> (usize, std::ops::RangeInclusive<usize>) {
        let seq_vec = self.get_ref_seq_vec();
        let _min = seq_vec
            .iter()
            .fold(usize::MAX, |_min, v_f32| min(_min, v_f32.len()));
        let _max = seq_vec
            .iter()
            .fold(usize::MIN, |_max, v_f32| max(_max, v_f32.len()));
        (self.get_meas_num(), _min..=_max)
    }
}

/// Adaptation of [`F32Vec2d`] for blood pressure measurements (sequence of 3 measurement vectors).
#[derive(Debug, Serialize, Deserialize)]
struct BpSequence(F32Vec2d);
impl BpSequence {
    fn new(len: usize) -> Self {
        Self(F32Vec2d::new_xy(3, len))
    }
    /// Get mut ref to internal sequence of measurements (`VecMeas2dType`)
    fn get_ref_seq_vec_mut(&mut self) -> &mut VecMeas2dType {
        self.0.get_ref_seq_vec_mut()
    }
    /// Get ref to internal sequence of measurements (`VecMeas2dType`)
    fn get_ref_seq_vec(&self) -> &VecMeas2dType {
        self.0.get_ref_seq_vec()
    }
    /// Get mut ref to internal measurement (`VecMeasType`) of sequence at index `idx_m`
    /// # Panic
    /// Panics if `idx_m` not is in range of valid indexes.
    fn get_ref_meas_vec_mut(&mut self, idx_m: usize) -> &mut VecMeasType {
        self.0.get_ref_meas_vec_mut(idx_m)
    }
    /// Get ref to internal measurement (`VecMeasType`) of sequence at index `idx_m`
    /// # Panic
    /// Panics if `idx_m` not is in range of valid indexes.
    fn get_ref_meas_vec(&self, idx_m: usize) -> &VecMeasType {
        self.0.get_ref_meas_vec(idx_m)
    }
    /// Returns *number* of measurement vectors in sequence.
    #[inline]
    fn get_meas_num(&self) -> usize {
        self.0.get_meas_num()
    }
    /// Returns *length* of measurement vectors in sequence.
    #[inline]
    fn get_meas_vec_len(&self) -> usize {
        self.0.get_meas_vec_len()
    }
    /// 1. Performs check if all measurement `vec`s are of the same length.
    /// 2. Tries to shrink the capacity of all measurement `vec`s.
    /// # Panic
    /// Panics if measurement `vec`s are not of the same length.
    fn validate_shrink(&mut self) {
        self.0.validate_shrink();
    }
    /// Sorts all measurement `vec`s individually.
    fn sort_seq(&mut self) {
        self.0.sort_seq();
    }
    /// Get the vector capacity in both dimensions:
    /// 1. Capacity of the sequence (`VecMeas2dType:Vec<VecMeasType>`)
    /// 2. Capacity range (`min..=max`) of the measurements (`VecMeasType:Vec<f32>`)
    fn get_dim_capacity(&self) -> (usize, std::ops::RangeInclusive<usize>) {
        self.0.get_dim_capacity()
    }
    /// Get the vector length in both dimensions:
    /// 1. Length of the sequence (`VecMeas2dType:Vec<VecMeasType>`)
    /// 2. Length range (`min..=max`) of the measurements (`VecMeasType:Vec<f32>`)
    fn get_dim_filled(&self) -> (usize, std::ops::RangeInclusive<usize>) {
        self.0.get_dim_filled()
    }
}

// ################################################################

/// Struct for importing entries from CSV file
#[derive(Debug, Deserialize, Clone)]
struct MeasCsv {
    date: String,
    time: String,
    sys: f32,
    dia: f32,
    pul: f32,
}
impl fmt::Display for MeasCsv {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{},{},{:.1},{:.1},{:.1}",
            self.date, self.time, self.sys, self.dia, self.pul
        )
    }
}
impl MeasCsv {
    /// Generates new 'MeasCsv' object for the given BP values at the current time
    pub fn new(sys: f32, dia: f32, pul: f32) -> Self {
        Self {
            date: get_date(),
            time: get_time(),
            sys,
            dia,
            pul,
        }
    }
    /// Returns the BP as array [`sys`, `dia`, `pul`]
    pub fn get_bp(&self) -> BpSampleType {
        [self.sys, self.dia, self.pul]
    }
    /// Returns the BP field `sys`
    pub fn get_bp_sys(&self) -> f32 {
        self.sys
    }
    /// Returns the BP field `dia`
    pub fn get_bp_dia(&self) -> f32 {
        self.dia
    }
    /// Returns the BP field `pul`
    pub fn get_bp_pul(&self) -> f32 {
        self.pul
    }
    /// Returns the `date` field
    pub fn get_date_str(&self) -> &str {
        &self.date[..]
    }
    /// Returns the `time` field
    pub fn get_time_str(&self) -> &str {
        &self.time[..]
    }
    /// Get `DateTime<Utc>` from `String` fields `date`, `time`
    pub fn get_datetime(&self) -> DateTime<Utc> {
        let dt_string = format!("{} {}", self.date, self.time);
        let dt = NaiveDateTime::parse_from_str(&dt_string, "%Y-%m-%d %H:%M:%S")
            .expect(&format!("Unable to read time '{dt_string}'!"))
            .and_utc();
        dt
    }
    /// Get `get_day_zero`, the day before the first day of the month
    pub fn get_day_zero(&self) -> DateTime<Utc> {
        // Create String for first day of current month
        let day_1_s = format!("{}-01 00:00:00", &self.date[..7]);

        // Get `DateTime` for first day of current month
        let day_1 = NaiveDateTime::parse_from_str(&day_1_s, "%F %T")
            .expect(&format!("Unable to read date '{}'!", day_1_s))
            .and_utc();

        // return `day_zero`
        day_1 - TimeDelta::days(1)
    }
    /// Create 'Meas2' object from this
    pub fn to_m2(&self, day_zero: DateTime<Utc>, interval: u8) -> Meas2 {
        Meas2::new(&self, day_zero, interval)
    }
}

/// Measurement-2 exists only as long as the ref MeasCsv
#[derive(Debug)]
struct Meas2 {
    bp_sample: BpSampleType,
    datetime: DateTime<Utc>,
    day_fine: f32,
    day_coarse: f32,
    sec_fine: i64,
    sec_coarse: i64,
}
impl Meas2 {
    pub fn new<'a>(meas_csv: &'a MeasCsv, day_zero: DateTime<Utc>, interval: u8) -> Meas2 {
        let mut m2 = Meas2 {
            bp_sample: meas_csv.get_bp(),
            datetime: meas_csv.get_datetime(),
            day_fine: 0_f32,
            day_coarse: 0_f32,
            sec_fine: 0,
            sec_coarse: 0,
        };
        m2.calc_time_rel(day_zero, interval);

        m2
    }
    /// Calc (and set) day/sec fine/coarse
    pub fn calc_time_rel(&mut self, day_zero: DateTime<Utc>, interval: u8) {
        let td: TimeDelta = *self.get_datetime() - day_zero;

        self.sec_fine = td.num_seconds();
        self.day_fine = self.sec_fine as f32 / SECS_IN_DAYS_F32;

        let mut day_coarse = (self.day_fine * interval as f32).floor() / interval as f32;
        day_coarse += 1_f32 / (2 * interval) as f32;
        self.day_coarse = day_coarse;

        self.sec_coarse = (day_coarse * SECS_IN_DAYS_F32) as i64;
    }
    /// Get field `day_fine`
    pub fn get_day_fine(&self) -> f32 {
        self.day_fine
    }
    /// Get field `sec_fine`
    pub fn get_sec_fine(&self) -> i64 {
        self.sec_fine
    }
    /// Get field `day_coarse`
    pub fn get_day_coarse(&self) -> f32 {
        self.day_coarse
    }
    /// Get field `sec_coarse`
    pub fn get_sec_coarse(&self) -> i64 {
        self.sec_coarse
    }
    /// Returns the field `datetime`
    pub fn get_datetime(&self) -> &DateTime<Utc> {
        &self.datetime
    }
    /// Returns the BP as array [`sys`, `dia`, `pul`]
    pub fn get_bp(&self) -> BpSampleType {
        self.bp_sample
    }
    /// Returns the BP field `sys`
    pub fn get_bp_sys(&self) -> f32 {
        self.bp_sample[0]
    }
    /// Returns the BP field `dia`
    pub fn get_bp_dia(&self) -> f32 {
        self.bp_sample[1]
    }
    /// Returns the BP field `pul`
    pub fn get_bp_pul(&self) -> f32 {
        self.bp_sample[2]
    }
}

#[derive(Debug)]
struct CollectionCsv {
    vec_csv: Vec<MeasCsv>,
}
impl CollectionCsv {
    pub fn new() -> Self {
        Self {
            vec_csv: Vec::new(),
        }
    }
    pub fn new_with_capacity(capacity: usize) -> Self {
        Self {
            vec_csv: Vec::with_capacity(capacity),
        }
    }
    /// Add (and consume) a `MeasCsv` object to vector field.
    pub fn add_csv_consume(&mut self, meas_csv: MeasCsv) {
        self.vec_csv.push(meas_csv);
    }
    /// Clear vector field.
    pub fn clear(&mut self) {
        self.vec_csv.clear();
    }
    /// Get ref to vector field.
    pub fn get_ref(&self) -> &Vec<MeasCsv> {
        &self.vec_csv
    }
    /// Sort collection vector by fields `date`, `time`
    pub fn sort(&mut self) {
        self.vec_csv
            .sort_by(|a, b| a.date.cmp(&b.date).then(a.time.cmp(&b.time)));
    }
    /// Create 'Meas2 collection' object from this
    pub fn to_coll_m2(&self, interval: u8) -> CollectionMeas2 {
        CollectionMeas2::from_coll_m1(self, interval)
    }
}

#[derive(Debug)]
struct CollectionMeas2 {
    // coll_csv: &'a CollectionCsv,
    vec_meas2: Vec<Meas2>,
    day_zero: DateTime<Utc>,
}
impl CollectionMeas2 {
    pub fn from_coll_m1(coll_csv: &CollectionCsv, interval: u8) -> CollectionMeas2 {
        let v_csv = &coll_csv.vec_csv;
        let v_len = v_csv.len();
        if v_len == 0 {
            panic!("'coll_csv' is empty!")
        }

        // Determine `day_zero` from first entry in 'CollectionCsv' obj
        let m_csv_1 = v_csv.first().unwrap();
        let day_zero = m_csv_1.get_day_zero();

        // Create 'CollectionMeas2' object
        let mut ret_coll_m2 = CollectionMeas2 {
            // coll_csv,
            vec_meas2: Vec::with_capacity(v_len),
            day_zero,
        };

        // Populate 'vec_meas2' of 'CollectionMeas2' object
        for m_csv in v_csv {
            let m2 = m_csv.to_m2(day_zero, interval);
            ret_coll_m2.add_meas2_consume(m2);
        }

        return ret_coll_m2;
    }
    /// Add (and consume) a `Meas2` object to vector field.
    fn add_meas2_consume(&mut self, meas_2: Meas2) {
        self.vec_meas2.push(meas_2);
    }
    /// Get mut ref to vector field.
    fn get_ref_mut(&mut self) -> &mut Vec<Meas2> {
        &mut self.vec_meas2
    }
    /// Get ref to vector field.
    pub fn get_ref(&self) -> &Vec<Meas2> {
        &self.vec_meas2
    }
    /// Get ref to day_zero.
    pub fn get_day0(&self) -> &DateTime<Utc> {
        &self.day_zero
    }
    /// Sort collection vector by field `datetime`
    pub fn sort(&mut self) {
        self.vec_meas2.sort_by_key(|k| k.datetime);
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CollectionDay {
    day: f32,
    sec: i64,
    sample_len: usize,
    bp_seq: BpSequence,
    analysis: AnalyzeDay,
    #[serde(skip)]
    completed: bool,
}
impl CollectionDay {
    /// Creates new `CollectionDay` obj from provided `Vec<Meas2>`.
    pub fn new_from_vec_m2(vec_m2: &Vec<Meas2>) -> Self {
        let m2_first = vec_m2.first().unwrap();

        // check if all sec_coarse in vector match
        Self::check_coarse_match(vec_m2, m2_first);

        let mut ret_cd = Self {
            day: m2_first.get_day_coarse(),
            sec: m2_first.get_sec_coarse(),
            sample_len: vec_m2.len(),
            bp_seq: BpSequence::new(vec_m2.len()),
            analysis: AnalyzeDayBuilder::build_empty(),
            completed: false,
        };

        for m2 in vec_m2 {
            ret_cd.add_meas2(m2);
        }
        ret_cd.bp_seq.validate_shrink();

        ret_cd.perform_analysis();

        ret_cd
    }
    /// Perform 'sort' on field `bp_seq:BpSequence` (sorts each measurement vector separatly).
    fn sort_seq(&mut self) {
        self.bp_seq.sort_seq();
    }
    /// Perform analysis based on `bp_seq:BpSequence` to create `AnalyzeDay` obj
    pub fn perform_analysis(&mut self) {
        self.analysis = AnalyzeDayBuilder::build(&mut self.bp_seq);
        self.set_completed();
    }
    /// Add singe measurement array `BpType` from `Meas2` obj to internal `bp_seq:BpSequence`.
    fn add_meas2(&mut self, m2: &Meas2) {
        self.check_can_edit();

        // get measurement
        let meas = m2.get_bp();
        // get mut ref to underlying `VecMeas2dType` obj of `bp_seq:BpSequence`
        let ref_seq = self.get_ref_mut();

        for idx_m in 0..3 {
            self.bp_seq.get_ref_meas_vec_mut(idx_m).push(meas[idx_m]);
        }
    }
    /// Returns ref to internal `Vec<BpType>` (`vec_bp`)
    pub fn get_ref_mut(&mut self) -> &mut VecMeas2dType {
        self.bp_seq.get_ref_seq_vec_mut()
    }
    /// Returns ref to internal `Vec<BpType>` (`vec_bp`)
    pub fn get_ref(&self) -> &VecMeas2dType {
        self.bp_seq.get_ref_seq_vec()
    }
    /// Returns `len()` of the internal `Vec<BpType>` (`vec_bp`).
    pub fn get_sample_size(&self) -> usize {
        self.bp_seq.get_meas_num()
    }
    /// Returns ref to grp struct `AnalyzeDay` obj.
    pub fn get_analysis_grp_ref(&self) -> &AnalyzeDay {
        &self.analysis
    }
    /// Returns ref to 'sys' `AnalyzeResult` obj.
    pub fn get_analysis_sys_ref(&self) -> &AnalyzeResult {
        &self.analysis.get_sys()
    }
    /// Returns ref to 'dia' `AnalyzeResult` obj.
    pub fn get_analysis_dia_ref(&self) -> &AnalyzeResult {
        &self.analysis.get_dia()
    }
    /// Returns ref to 'pul' `AnalyzeResult` obj.
    pub fn get_analysis_pul_ref(&self) -> &AnalyzeResult {
        &self.analysis.get_pul()
    }

    pub fn set_completed(&mut self) {
        self.completed = true;
    }
    pub fn get_completed(&self) -> bool {
        self.completed
    }
    /// Check if all objs in `Vec<Meas2>` have the same `sec_coarse` field as `m2_first` obj
    /// # Panic
    /// Will panic, if any element in `vec_m2` has a different field `sec_coarse` to the provided `m2_first` obj
    pub fn check_can_edit(&self) {
        if self.get_completed() {
            panic!("Obj is set to 'completed'!")
        }
    }
    /// Check if all objs in `Vec<Meas2>` have the same `sec_coarse` field as `m2_first` obj
    /// # Panic
    /// Will panic, if any element in `vec_m2` has a different field `sec_coarse` to the provided `m2_first` obj
    fn check_coarse_match(vec_m2: &Vec<Meas2>, m2_first: &Meas2) {
        let coarse_match = vec_m2
            .iter()
            .all(|x| x.get_sec_coarse() == m2_first.get_sec_coarse());
        if !coarse_match {
            panic!(
                "Mismatch of `sec_coarse`s in `Vec<Meas2>` ({:?})!",
                m2_first.get_sec_coarse()
            )
        }
    }
}

#[derive(Serialize, Deserialize, DebugPretty)]
struct CollectionMonth {
    day_zero: DateTimeSimple,
    hash_map: CollDayHashType,
}
impl CollectionMonth {
    /// Create empty `CollectionMonth` obj
    pub fn new() -> Self {
        Self {
            day_zero: DateTimeSimple::new(),
            hash_map: HashMap::new(),
        }
    }
    /// Create new `CollectionMonth` obj from `CollectionMeas2` obj
    pub fn from_coll_m2_consume(mut coll_m2: CollectionMeas2) -> Self {
        let mut ret_cm = Self::new();
        ret_cm.set_day_zero(coll_m2.get_day0());

        coll_m2.sort();
        let vec_m2 = coll_m2.get_ref_mut();

        // let mut wdt = 100;
        while vec_m2.len() > 0 {
            // wdt -= 1;
            let sec = vec_m2.first().unwrap().get_sec_coarse();

            let vec_extracted = vec_m2
                .extract_if(.., |x| x.sec_coarse == sec)
                .collect::<Vec<_>>();
            // println!(
            //     "Extracted {}. Remaining {}.",
            //     vec_extracted.len(),
            //     vec_m2.len()
            // )
            ret_cm.add_vec_m2(vec_extracted);
        }
        // println!("[DEBUG] Extraction End. wdt={wdt}");

        ret_cm.finish();

        return ret_cm;
    }

    pub fn set_day_zero(&mut self, day_0: &DateTime<Utc>) {
        self.day_zero.set_utc(&day_0);
    }
    /// Add contents of `Meas2` obj to internal `HashMap`.
    pub fn add_vec_m2(&mut self, vec_m2: Vec<Meas2>) {
        let sec = vec_m2.first().unwrap().get_sec_coarse();

        // Check if key `sec` is already in `HashMap`
        match self.get_ref_mut().get_mut(&sec) {
            Some(_) => {
                panic!("`CollectionDay ` for key `{sec}` already in HashMap!")
            }
            None => {
                // Create new `CollectionDay` obj *and* add measurement into it.
                // (This is handled by `CollectionDay::new_from_m2` directly!!!)
                // Then insert to `HashMap`.
                self.get_ref_mut()
                    .insert(sec, CollectionDay::new_from_vec_m2(&vec_m2));
            }
        }
    }
    /// Perform analysis on all `CollectionDay` obj in `HashMap`
    pub fn finish(&mut self) {
        for k in self.get_key_sorted() {
            let day = self.get_ref_mut().get_mut(&k).unwrap();
            day.perform_analysis();
        }
    }
    // /// Clears internal `HashMap`
    // pub fn clear(&mut self) {
    //     self.coll_day_map.clear();
    // }
    /// Returns mut ref to internal `HashMap`
    pub fn get_ref_mut(&mut self) -> &mut CollDayHashType {
        &mut self.hash_map
    }
    /// Returns ref to internal `HashMap`
    pub fn get_ref(&self) -> &CollDayHashType {
        &self.hash_map
    }
    /// Returns 'length' of the `CollectionDay` obj corresponding to the key `k` (`i64`).\
    /// The 'length' of the `CollectionDay` obj is `len()` of its internal `Vec<(BpType)>`.
    ///
    /// `Option<usize>`: Returns `Some(usize)`, if key `k` in map, `None` otherwise.
    pub fn get_sample_size(&self, k: i64) -> Option<usize> {
        match self.get_ref().get(&k) {
            Some(v) => Some(v.get_sample_size()),
            None => None,
        }
    }
    /// Get all keys (`i64`) from internal `HashMap` , sorted as `Vec<i64>`.
    pub fn get_key_sorted(&self) -> Vec<i64> {
        let mut vec_keys: Vec<i64> = self.get_ref().keys().map(|x| x.clone()).collect();
        vec_keys.sort();
        vec_keys
    }
}

#[derive(Debug)]
struct AnalyzeDayBuilder {}
impl AnalyzeDayBuilder {
    pub fn build<'a>(bp_seq: &'a mut BpSequence) -> AnalyzeDay {
        let mut ret_item = AnalyzeDay([
            AnalyzeResult::new("sys"),
            AnalyzeResult::new("dia"),
            AnalyzeResult::new("pul"),
        ]);

        // sort vectors in sequence
        bp_seq.sort_seq();

        // Get Q0,Q4 (min/max )
        Self::calc_min_max(&mut ret_item, bp_seq);

        // Calc Q1,Q2,Q3 & IQR & Whiskers
        Self::calc_quartile(&mut ret_item, bp_seq);

        return ret_item;
    }
    pub fn build_empty() -> AnalyzeDay {
        AnalyzeDay([
            AnalyzeResult::new("sys"),
            AnalyzeResult::new("dia"),
            AnalyzeResult::new("pul"),
        ])
    }
    // fn create_samples<'a>(vec_bp: &'a Vec<BpType>) -> [Vec<f32>; 3] {
    //     let len = vec_bp.len();
    //     let mut a_measurement: [Vec<f32>; 3] = [
    //         Vec::<f32>::with_capacity(len),
    //         Vec::<f32>::with_capacity(len),
    //         Vec::<f32>::with_capacity(len),
    //     ];
    //     // Insert all samples into their respective `Vec`
    //     for bp_line in vec_bp {
    //         a_measurement[0].push(bp_line[0]);
    //         a_measurement[1].push(bp_line[1]);
    //         a_measurement[2].push(bp_line[2]);
    //     }
    //     // Sort all sample `Vec`s
    //     for mut vec_x in &mut a_measurement {
    //         vec_x.sort_by(f32::total_cmp);
    //     }
    //     return a_measurement;
    // }
    fn calc_min_max(ret_item: &mut AnalyzeDay, bp_seq: &BpSequence) {
        let bp_vec2d = bp_seq.get_ref_seq_vec();

        for idx_m in 0..bp_seq.get_meas_num() {
            // get sample vector for this measurement
            let vec_x = bp_seq.get_ref_meas_vec(idx_m);
            // get internal `AnalyzeResult` obj
            let res = &mut ret_item.0[idx_m];
            // get min and max
            res.quartile[0] = vec_x.first().unwrap().clone();
            res.quartile[4] = vec_x.last().unwrap().clone();
        }
    }
    /// Calculates the quartiles Q1,Q2,Q3 and checks for outliers.
    ///
    /// `q(p) = x[k] + a(x[k+1] - x[k])`
    ///
    /// With
    /// - `k = floor( p*(N-1) )` [^1]
    /// - `a = p*(N-1) - k` [^1]
    ///
    /// Where
    /// - `N`: sample size
    /// - `p`: quartile percentage
    /// - `x[k]`: sample at index `k`
    /// - `Q1...Q3`: `q(p)` for `p = [0.25, 0.5, 0.75]`
    ///
    /// [^1]: The Source uses `N+1` here for some unknown reason.
    ///
    /// # Source
    /// Dekking, Michel (2005). *A modern introduction to probability and statistics: understanding why and how.*\
    /// London: Springer. **pp. 236-238**. ISBN 978-1-85233-896-1. OCLC 262680588.\
    /// https://archive.org/details/modernintroducti0000unse_h6a1
    ///
    /// ## The case for `N-1`
    /// The source's examples seem to use one-based indexing `[1,N]`, but their unmodified equation \
    /// for `k(p,N) = floor( p*(N+1) )` will result in out-of-bounds indexing:
    /// - `k = 0` for `p < 0.01` and
    /// - `k+1 = N+1`for `p > 0.99`.
    ///
    /// Modifying the equation for `k` (and `a`) to use `N-1` instead of `N+1` causes the results \
    /// for `k` and `k+1` to stay inside the interval `[0,N-1]`, which is fitting for zero-based indexing.
    fn calc_quartile(ret_item: &mut AnalyzeDay, bp_seq: &BpSequence) {
        let vec_len: usize = bp_seq.get_meas_vec_len();

        // Array of quartile percentages p
        let a_p: [f32; _] = [0.0, 0.25, 0.5, 0.75, 1.0];
        // Array of indexes of `a_p` to consider for calculation (Q1,Q2,Q3)
        let a_idx_p: [usize; _] = [1, 2, 3];

        // Go through Q1,Q2,Q3 (25%,50%,75%)
        'Loop_Q123: for idx_p in a_idx_p {
            // get current percentage and calc parameter
            let p = a_p[idx_p];
            let (k, a) = Self::calc_quartile_param(vec_len, p);

            // For sys,dia,pul: Calc Quartile for current percentage `p`
            'Loop_SysDiaPul_1: for idx_m in 0..bp_seq.get_meas_num() {
                // get sample vector for this measurment
                let vec_x = bp_seq.get_ref_meas_vec(idx_m);
                // get `AnalyzeResult` obj for this measurment
                let res = &mut ret_item.0[idx_m];
                // calculate Quartile for current percentage `p`
                if vec_x.len() > 1 {
                    res.quartile[idx_p] = vec_x[k] + a * (vec_x[k + 1] - vec_x[k]);
                } else {
                    // excemption for short sample vectors
                    res.quartile[idx_p] = vec_x[k];
                }
            }
        }

        // For sys,dia,pul: Calc IQR,whiskers
        'Loop_SysDiaPul_2: for idx_m in 0..bp_seq.get_ref_seq_vec().len() {
            // get sample vector for this measurment
            let vec_x = bp_seq.get_ref_meas_vec(idx_m);
            // get `AnalyzeResult` obj for this measurment
            let res = &mut ret_item.0[idx_m];
            // Calc IQR (interquartile range)
            res.iqr = res.quartile[3] - res.quartile[1];

            // Calc upper/lower whisker limits
            // Check for outliers outside whiskers
            Self::check_outlier_upper(res, vec_x);
            Self::check_outlier_lower(res, vec_x);
        }
    }
    /// Calculates the parameters `k` and `a` for the calculation of the quartiles.
    ///
    /// - `k = floor( p*(N-1) )` [^1]
    /// - `a = p*(N-1) - k` [^1]
    ///
    /// Where
    /// - `N`: sample size
    /// - `p`: quartile percentage
    ///
    /// [^1]: The Source uses `N+1` here for some unknown reason.
    ///
    /// # Returns
    /// - `k: usize`
    /// - `a: f32`
    ///
    /// # Source
    /// Dekking, Michel (2005). *A modern introduction to probability and statistics: understanding why and how.*\
    /// London: Springer. **pp. 236-238**. ISBN 978-1-85233-896-1. OCLC 262680588.\
    /// https://archive.org/details/modernintroducti0000unse_h6a1
    ///
    /// ## The case for `N-1`
    /// The source's examples seem to use one-based indexing `[1,N]`, but their unmodified equation \
    /// for `k(p,N) = floor( p*(N+1) )` will result in out-of-bounds indexing:
    /// - `k = 0` for `p < 0.01` and
    /// - `k+1 = N+1`for `p > 0.99`.
    ///
    /// Modifying the equation for `k` (and `a`) to use `N-1` instead of `N+1` causes the results \
    /// for `k` and `k+1` to stay inside the interval `[0,N-1]`, which is fitting for zero-based indexing.
    fn calc_quartile_param(sample_len: usize, p: f32) -> (usize, f32) {
        let temp = p * (sample_len - 1) as f32;
        let _k = temp.floor();

        let k = _k as usize;
        let a = temp - _k;
        (k, a)
    }
    fn check_outlier_upper(res: &mut AnalyzeResult, vec_x: &VecMeasType) {
        let len = vec_x.len();
        let lim_u = res.quartile[3] + 1.5 * res.iqr;

        if lim_u >= res.get_max() {
            // There are no outliers above Median: upper whisker <- max
            res.whisker_upper = res.get_max();
        } else {
            // There must be some outliers above Median!
            // Go through all samples `x` in sample `Vec` `vec_x` in descending order:
            // - If `x > lim_u`, then add it to outlier `Vec`
            // - Else: The upper whisker border is found!
            'Loop_sample_desc: for idx_desc in (0..len).rev() {
                let x = vec_x[idx_desc];
                if x > lim_u {
                    // Add to outliers
                    res.outliers.push(x);
                } else {
                    // Upper whisker border found
                    res.whisker_upper = x;
                    break; // And stop searching
                }
            }
        }
    }
    fn check_outlier_lower(res: &mut AnalyzeResult, vec_x: &VecMeasType) {
        let len = vec_x.len();
        let lim_l = res.quartile[1] - 1.5 * res.iqr;

        if lim_l <= res.get_min() {
            // There are no outliers below Median: lower whisker <= min
            res.whisker_lower = res.get_min();
        } else {
            // There must be some outliers below Median!
            // Go through all samples `x` in sample `Vec` `vec_x` in ascending order:
            // - If `x < lim_l`, then add it to outlier `Vec`
            // - Else: The lower whisker border is found!
            'Loop_sample_asc: for idx_asc in 0..len {
                let x = vec_x[idx_asc];
                if x < lim_l {
                    // Add to outliers
                    res.outliers.push(x);
                } else {
                    // Lower whisker border found
                    res.whisker_lower = x;
                    break; // And stop searching
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnalyzeDay([AnalyzeResult; 3]);
impl AnalyzeDay {
    pub fn get_sys(&self) -> &AnalyzeResult {
        &self.0[0]
    }
    pub fn get_dia(&self) -> &AnalyzeResult {
        &self.0[1]
    }
    pub fn get_pul(&self) -> &AnalyzeResult {
        &self.0[2]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnalyzeResult {
    name: String,
    quartile: [f32; 5],
    outliers: Vec<f32>,
    iqr: f32,
    whisker_upper: f32,
    whisker_lower: f32,
}
impl AnalyzeResult {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            quartile: [0_f32; 5],
            outliers: Vec::new(),
            iqr: 0_f32,
            whisker_upper: 0_f32,
            whisker_lower: 0_f32,
        }
    }
    fn get_name(&self) -> &String {
        &self.name
    }
    fn get_median(&self) -> f32 {
        self.quartile[2]
    }
    fn get_iqr(&self) -> f32 {
        self.iqr
    }
    fn get_min(&self) -> f32 {
        self.quartile[0]
    }
    fn get_max(&self) -> f32 {
        self.quartile[4]
    }
    fn get_q0(&self) -> f32 {
        self.quartile[0]
    }
    fn get_q1(&self) -> f32 {
        self.quartile[1]
    }
    fn get_q2(&self) -> f32 {
        self.quartile[2]
    }
    fn get_q3(&self) -> f32 {
        self.quartile[3]
    }
    fn get_q4(&self) -> f32 {
        self.quartile[4]
    }
    fn get_whisker_upper(&self) -> f32 {
        self.whisker_upper
    }
    fn get_whisker_lower(&self) -> f32 {
        self.whisker_lower
    }
    fn get_outlier(&self) -> &Vec<f32> {
        &self.outliers
    }
    fn has_outlier(&self) -> bool {
        self.outliers.len() > 0
    }
}

#[derive(Serialize, Deserialize, DebugPretty)]
#[allow(non_snake_case)]
struct DateTimeSimple {
    timestamp: i64,
    Y: i32,
    m: u32,
    d: u32,
    H: u32,
    M: u32,
    S: u32,
}
impl DateTimeSimple {
    fn new() -> Self {
        Self {
            timestamp: 0,
            Y: 0,
            m: 0,
            d: 0,
            H: 0,
            M: 0,
            S: 0,
        }
    }
    fn from_utc(date_time_utc: DateTime<Utc>) -> Self {
        Self {
            timestamp: date_time_utc.timestamp(),
            Y: date_time_utc.year(),
            m: date_time_utc.month(),
            d: date_time_utc.day(),
            H: date_time_utc.hour(),
            M: date_time_utc.minute(),
            S: date_time_utc.second(),
        }
    }
    fn set_utc(&mut self, date_time_utc: &DateTime<Utc>) {
        self.timestamp = date_time_utc.timestamp();
        self.Y = date_time_utc.year();
        self.m = date_time_utc.month();
        self.d = date_time_utc.day();
        self.H = date_time_utc.hour();
        self.M = date_time_utc.minute();
        self.S = date_time_utc.second();
    }
}

#[derive(Serialize, Deserialize, DebugPretty)]
struct FileHandler {
    path_dir: PathBuf,
    path_file: PathBuf,
}
impl FileHandler {
    pub fn new(directory: &str, file: &str) -> Self {
        let p_dir = Path::new(&directory).to_owned();
        let p_file = Path::new(&directory).join(&file).to_owned();

        let mut ret_obj = Self {
            path_dir: p_dir,
            path_file: p_file,
        };
        ret_obj
    }
    fn get_path_dir(&self) -> &PathBuf {
        &self.path_dir
    }
    fn get_path_file(&self) -> &PathBuf {
        &self.path_file
    }
    /// Checks if directory exists and tries to create it, if not.
    pub fn check_directory(&self) -> Result<(), std::io::Error> {
        let path_dir = self.get_path_dir();
        if path_dir.exists() {
            return Ok(());
        }
        log_warning(&format!("Directory '{:?}' missing.", path_dir,));

        match fs::create_dir(path_dir) {
            Ok(_) => {
                log_message(&format!("Directory '{:?}' created.", path_dir));
                Ok(())
            }
            Err(e) => {
                log_error(&format!("Unable to create directory '{:?}'.", path_dir));
                Err(e)
            }
        }
    }
    /// Checks if file exists.
    ///
    /// | Case                     | Returns                                 |
    /// | ------------------------ | --------------------------------------- |
    /// | File does not exist      | `Ok( FileState::Missing )`              |
    /// | File exists              | `Ok( FileState::Exists(filesize:u64) )` |
    /// | Missing file permissions | `std::io::Error`                        |
    pub fn check_file_exists(&self) -> Result<FileState, std::io::Error> {
        let path_file = self.get_path_file();

        if !path_file.exists() {
            return Ok(FileState::Missing);
        }
        match fs::metadata(path_file) {
            Ok(metadata) => {
                return Ok(FileState::Exists(metadata.len()));
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    pub fn file_open(&self, mode: FileOpenMode) -> File {
        let path_file = self.get_path_file();

        let fh: File;
        match mode {
            FileOpenMode::Read => {
                fh = OpenOptions::new()
                    .read(true)
                    .open(path_file)
                    .expect(&format!(
                        "Unable to open file '{:?}' in {:?}.",
                        path_file, mode
                    ));
            }
            FileOpenMode::WriteReset => {
                fh = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(path_file)
                    .expect(&format!(
                        "Unable to open file '{:?}' in {:?}.",
                        path_file, mode
                    ));
            }
            FileOpenMode::WriteAppend => {
                fh = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .open(path_file)
                    .expect(&format!(
                        "Unable to open file '{:?}' in {:?}.",
                        path_file, mode
                    ));
            }
        }
        return fh;
    }
}

// ################################################################

#[derive(Debug, PartialEq)]
enum CsvOpenMode {
    Read,
    WriteReset,
    WriteAppend,
}

#[derive(Debug, PartialEq)]
enum FileOpenMode {
    Read,
    WriteReset,
    WriteAppend,
}

#[derive(Debug, PartialEq)]
enum FileState {
    Missing,
    Exists(u64),
}

// ################################################################
// ################################################################
fn main() {
    let cli = Cli::parse();
    println!("CLI: {:?}\n", &cli);

    worker_init_csv();

    match cli.add {
        Some(bp) => worker_bp_add(&bp),
        _ => (),
    }

    if cli.rebuild || cli.status || cli.output {
        let csv_collection = read_csv_content().expect("Unable to perform 'Read of CSV File'.");

        if cli.rebuild {
            worker_csv_rebuild(&csv_collection);
        }
        if cli.output {
            worker_output(&csv_collection);
        }
        if cli.status {
            worker_csv_status(&csv_collection);
        }
    }

    return;
}

/// Will read the CSV file, sort measurements and overwrite the file
fn worker_output(csv_collection: &CollectionCsv) {
    let coll_m2 = csv_collection.to_coll_m2(2);

    let coll_month = CollectionMonth::from_coll_m2_consume(coll_m2);

    println!("{coll_month:?}");
    if true {
        return;
    }
    println!("Attempt json export");
    let pretty = serde_json::to_string_pretty(&coll_month).unwrap(); // pretty-printed
    println!("{pretty}");

    // // println!("{:?}", cm.get_ref());
    // let hm = coll_month.get_ref();
    // let keys = coll_month.get_key_sorted();
    // println!("keys (# {}): {:?}", keys.len(), keys);

    // for k in keys {
    //     if let Some(cd) = hm.get(&k) {
    //         let size = cd.get_sample_size();
    //         let mut r = cd.get_analysis_sys_ref();
    //         let t_sys = format!(
    //             "[{}] wL: {:3.0}, Q1: {:5.1}, Q2: {:5.1}, Q3: {:5.1}, wU: {:3.0}, IQR: {:5.1}, #out: {}",
    //             r.get_name(),
    //             r.get_whisker_lower(),
    //             r.get_q1(),
    //             r.get_q2(),
    //             r.get_q3(),
    //             r.get_whisker_upper(),
    //             r.get_iqr(),
    //             r.get_outlier().len()
    //         );
    //         r = cd.get_analysis_dia_ref();
    //         let t_dia = format!(
    //             "[{}] wL: {:3.0}, Q1: {:5.1}, Q2: {:5.1}, Q3: {:5.1}, wU: {:3.0}, IQR: {:5.1}, #out: {}",
    //             r.get_name(),
    //             r.get_whisker_lower(),
    //             r.get_q1(),
    //             r.get_q2(),
    //             r.get_q3(),
    //             r.get_whisker_upper(),
    //             r.get_iqr(),
    //             r.get_outlier().len()
    //         );
    //         r = cd.get_analysis_pul_ref();
    //         let t_pul = format!(
    //             "[{}] wL: {:3.0}, Q1: {:5.1}, Q2: {:5.1}, Q3: {:5.1}, wU: {:3.0}, IQR: {:5.1}, #out: {}",
    //             r.get_name(),
    //             r.get_whisker_lower(),
    //             r.get_q1(),
    //             r.get_q2(),
    //             r.get_q3(),
    //             r.get_whisker_upper(),
    //             r.get_iqr(),
    //             r.get_outlier().len()
    //         );
    //         println!("\n[{k}, {:.2}] #Samples: {}", cd.day, cd.get_sample_size());
    //         println!("\t{}", t_sys);
    //         println!("\t{}", t_dia);
    //         println!("\t{}", t_pul);
    //     }
    // }
}

/// Will read the CSV file, sort measurements and overwrite the file
fn worker_csv_rebuild(csv_collection: &CollectionCsv) {
    let path_string = get_file_path_string();
    // Open CSV File and reset content
    let fh_csv = open_csv_file(&path_string, CsvOpenMode::WriteReset);

    // println!("Sorted entries:");
    // for entry in &csv_entries {
    for (index, entry) in csv_collection.get_ref().iter().enumerate() {
        // let csv_line = entry.get_csv_entry_string();
        // println!("[{}] {:?}", index, csv_line);

        writeln!(&fh_csv, "{}", entry).expect(&format!(
            "Could not write to File '{path_string}': Entry [{index}] '{entry}'."
        ));
    }

    // Save changes to disk
    fh_csv
        .sync_all()
        .expect(&format!("Unable to save File '{path_string}'."));
}

fn worker_csv_status(csv_collection: &CollectionCsv) {
    let csv_entries = csv_collection.get_ref();
    let date_ym = get_date_ym();

    let csv_len = csv_entries.len();
    println!("File for '{date_ym}' contains {csv_len} entries.");

    if csv_len > 0 {
        let csv_tail_range = if csv_len <= 5 {
            0..csv_len
        } else {
            (csv_len - 5)..csv_len
        };

        println!("Latest entries:");
        for i in csv_tail_range {
            println!("[{}] {:?}", i + 1, csv_entries[i]);
        }
    }
}

fn worker_bp_add(bp: &Vec<f32>) {
    let sys = bp[0];
    let dia = bp[1];
    let pul = bp[2];

    let measurement = MeasCsv::new(sys, dia, pul);

    let path_string = get_file_path_string();

    // Open CSV file in 'append' mode
    let fh_csv = open_csv_file(&path_string, CsvOpenMode::WriteAppend);

    // Append entry to CSV file
    if let Err(e) = writeln!(&fh_csv, "{}", measurement) {
        eprintln!("Couldn't write to file: {}", e);
    }

    // Save changes to disk
    fh_csv
        .sync_all()
        .expect(&format!("Unable to save File '{path_string}'."));
}

fn worker_init_csv() {
    let path_dir_string = get_dir_path_string();
    let path_file_string = get_file_path_string();

    check_path(&path_dir_string).expect(&format!(
        "Unable to perform 'Check of Directory {path_dir_string}'."
    ));
    check_file(&path_file_string).expect(&format!(
        "Unable to perform 'Check of work File {path_file_string}'."
    ));
}

// ################################################################

fn open_csv_file(path_str: &str, mode: CsvOpenMode) -> File {
    let path_file = Path::new(path_str);

    let fh_csv: File;
    match mode {
        CsvOpenMode::Read => {
            fh_csv = OpenOptions::new()
                .read(true)
                .open(path_file)
                .expect(&format!(
                    "Unable to open File '{}' in {:?}.",
                    path_str, mode
                ));
        }
        CsvOpenMode::WriteReset => {
            fh_csv = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path_file)
                .expect(&format!(
                    "Unable to open File '{}' in {:?}.",
                    path_str, mode
                ));

            // Write CSV header line
            writeln!(&fh_csv, "{}", CSV_HEADER).expect(&format!(
                "Could not write to File '{}' in {:?}.",
                path_str, mode
            ));
        }
        CsvOpenMode::WriteAppend => {
            fh_csv = OpenOptions::new()
                .write(true)
                .append(true)
                .open(path_file)
                .expect(&format!(
                    "Unable to open File '{}' in {:?}.",
                    path_str, mode
                ));
        }
    }
    fh_csv
}

fn read_csv_content() -> Result<CollectionCsv, Box<dyn std::error::Error>> {
    let path_string = get_file_path_string();

    let fh_csv = open_csv_file(&path_string, CsvOpenMode::Read);
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b',')
        .from_reader(fh_csv);

    // let records_iter = rdr.deserialize();

    let records: Vec<Result<MeasCsv, csv::Error>> = rdr.deserialize().collect();
    // let mut ret: Vec<MeasCsv> = Vec::with_capacity(records.len());
    let mut coll = CollectionCsv::new_with_capacity(records.len());

    for result in records {
        let entry: MeasCsv = result?;
        // println!("{:?}", entry);
        // ret.push(entry);
        coll.add_csv_consume(entry);
    }

    // // Sort vector of Measurement by date, time
    // ret.sort_by(|a, b| a.date.cmp(&b.date).then(a.time.cmp(&b.time)));
    coll.sort();

    Ok(coll)
}

fn check_file(path_file_str: &str) -> Result<(), std::io::Error> {
    // let path_string = get_file_path_string();
    let path_file = Path::new(path_file_str);
    let fh: File;

    if path_file.exists() {
        let file_meta = fs::metadata(path_file).expect("unable to read metadata");
        // println!("metadata len: {:?}", file_meta.len());

        if file_meta.len() > 0 {
            let f_read = File::open(path_file)?;
            let reader = BufReader::new(f_read);

            let mut lines = reader.lines();
            let line = lines
                .next()
                .expect("Unable to read first Line of File")
                .expect("Unable to read first Line of File");

            if CSV_HEADER == &line[..] {
                return Ok(());
            } else {
                panic!(
                    "File '{}' has content, but is missing csv header!",
                    path_file_str
                );
            }
        }
        log_message(&format!(
            "Empty File '{}' missing csv header.",
            path_file_str
        ));

        fh = File::create(path_file)?;
    } else {
        log_warning(&format!("File '{}' missing.", path_file_str));

        fh = File::create_new(path_file)?;
        log_message(&format!("Empty File '{}' created.", path_file_str));
    }

    if let Err(e) = writeln!(&fh, "{}", CSV_HEADER) {
        eprintln!("Could not write to File: {}", e);
    }

    fh.sync_all()?;
    log_message(&format!("Csv header added to File '{}'.", path_file_str));

    Ok(())
}

fn check_path(path_dir_str: &str) -> std::io::Result<()> {
    let path_dir = Path::new(path_dir_str);

    if path_dir.exists() {
        return Ok(());
    }
    log_warning(&format!("Directory '{}' missing.", path_dir_str));

    match fs::create_dir(path_dir) {
        Ok(_) => {
            log_message(&format!("Directory '{}' created.", path_dir_str));
            Ok(())
        }
        Err(e) => {
            log_error(&format!("Unable to create Directory '{}'.", path_dir_str));
            Err(e)
        }
    }
}

fn log_message(msg: &str) {
    println!("[MESSAGE]\t{msg}");
}
fn log_warning(wrn: &str) {
    println!("[WARNING]\t{wrn}");
}
fn log_error(err: &str) {
    println!("[ERROR]\t{err}");
}

fn get_date() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}
fn get_date_ym() -> String {
    Local::now().format("%Y-%m").to_string()
}
fn get_time() -> String {
    Local::now().format("%H:%M:%S").to_string()
}

fn get_file_path_string() -> String {
    format!("./data/{}.csv", get_date_ym())
}
fn get_dir_path_string() -> String {
    format!("./data")
}
