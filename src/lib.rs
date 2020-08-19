pub mod bdf_reader {
    extern crate clap;
    extern crate byteorder;

    use std::collections::HashMap;
    use std::io::prelude::*;
    use std::io::SeekFrom;
    use std::fs::File;
    use std::process::exit;

    use crate::byte_reader::ByteReader;
    use crate::parse_helpers::*;

    type DynErr<T> = Result<T, Box<dyn std::error::Error>>;
    type Channels = HashMap<ChannelLabel, Vec<i32>>;
    type Body = Channels;


    pub struct BDF {
        pub header: Header,
        pub body: Body,
    }

    impl BDF {
        pub fn parse(file_path: &str) -> DynErr<BDF> {
            // Get file and set seek head to first byte
            let bdf_file_path = file_path;
            let mut bdf_file = File::open(bdf_file_path)?;

            let header  = match parse_header(&mut bdf_file) {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("Error occurred parsing BDF Header, exiting:\n{:#?}", err);
                    exit(1)
                }
            };
                    

            let mut channels = HashMap::new();

            //let _test_channel = match parse_body(&header, &mut bdf_file, &mut body_buffer, &mut channels) {
            let _ = match parse_body(&header, &mut bdf_file, &mut channels) {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("Error occured parsing BDF Body (Channel data), exiting:\n{:#?}", err);
                    exit(2)
                }
            };

            Ok(
                BDF {
                    header,
                    body: channels,
                }
            )
        }
    }

    fn parse_body<'a>(header: &'a Header, input_file: &'a mut File, channels: &'a mut HashMap<ChannelLabel, Vec<i32>>) -> DynErr<bool> {
        //move seek head to body of file
        input_file.seek(SeekFrom::Start(header.get_header_size() as u64))?;

        // each sample is 3 bytes (little endian)
        // example let number: i32 = Cursor::new(vec![0, 0, 255]).read_i24::<LittleEndian>().ok().unwrap();
        // duration is the number of sample sets for a given channel
        // one channel per record has duration * sample size
        //  or data_records_total (when not specified) = duration * sample_size * 16 (bytes)

        let dur = header.get_duration() as usize;
        //let metadata = header.channel_metadata.get_channels()
            //.collect::<Vec<Channel>>();

        for _ in 0..header.get_record_size() {
            for ch in header.channel_metadata.get_channels() {
                // get sample rate for current channel
                let sr = ch.sample_rate as usize;

                // raw byte data
                let mut channel_data = vec![0; sr * dur * 24];
                let _ = input_file.read(&mut channel_data);

                // convert and add to hashmap
                let mut value = chunk_little_endian(&channel_data, 3);
                let channel = channels.entry(String::from(&ch.label)).or_insert(Vec::new());
                (*channel).append(&mut value);

                // update head position
                input_file.seek(SeekFrom::Start(header.get_header_size() as u64))?;
            }
        }

        Ok(true)
    }

    fn parse_header(input_file: &mut File) -> DynErr<Header> {
        input_file.seek(SeekFrom::Start(0))?;

        // create a buffer and parse fixed section of the header
        let mut header_fixed_data = [0; 256];
        let _ = input_file.read(&mut header_fixed_data);
        let header_fixed = parse_fixed_header(&header_fixed_data);

        // move seek head to new position in header
        input_file.seek(SeekFrom::Start(256))?;

        // create a buffer and parse the dynamic section of the header
        let mut header_dynamic_data = vec![0; header_fixed.header_size - 256];
        let _ = input_file.read(&mut header_dynamic_data);
        let header_dynamic = parse_dynamic_header(&header_dynamic_data, header_fixed.channel_total);

        Ok(
            Header {
                file_metadata: header_fixed,
                channel_metadata: header_dynamic,
            }
        )
    }

    fn parse_fixed_header(input: &[u8]) -> FileMetadata {

        let mut header = ByteReader { byte_data: input, head_position: 0 as usize };

        FileMetadata {
            special_bit: header.next(1)[0],
            biosemi: string_from(header.next(7)),
            subject_id: string_from(header.next(80)),
            recording_id: string_from(header.next(80)),
            record_start_date: string_from(header.next(8)),
            record_start_time: string_from(header.next(8)),
            header_size: string_from(header.next(8)).parse::<usize>().unwrap(),
            version: string_from(header.next(44)),
            records_total: string_from(header.next(8)).parse::<usize>().unwrap(),
            record_duration: string_from(header.next(8)).parse::<usize>().unwrap(),
            channel_total: string_from(header.next(4)).parse::<usize>().unwrap(),
        }
    }

    fn parse_dynamic_header(input: &[u8], channel_total: usize) -> ChannelMetadata {
        let mut header = ByteReader { byte_data: input, head_position: 0 as usize };

        // dynamic header
        let mut dynamic_header = ChannelMetadata {
            channel_metadata: HashMap::new(),
            reserved: vec![],
        };

        // alias
        let ct = channel_total;

        // Channel data values
        let mut channel_labels = chunk_string(header.next(ct * 16), 16);
        let mut transducer_type = chunk_string(header.next(ct * 80), 80);
        let mut physical_dimension = chunk_string(header.next(ct * 8), 8);
        let mut unit_minimum = chunk_i64(header.next(ct * 8), 8);
        let mut unit_maximum = chunk_i64(header.next(ct * 8), 8);
        let mut digital_minimum = chunk_i64(header.next(ct * 8), 8);
        let mut digital_maximum = chunk_i64(header.next(ct * 8), 8);
        let mut prefilter = chunk_string(header.next(ct * 80), 80);
        let mut sample_rate = chunk_u64(header.next(ct * 8), 8);

        for _ in 0..channel_total {
            let label = channel_labels.pop().unwrap();
            dynamic_header.channel_metadata.insert(
                String::from(&label),
                Channel {
                    label: String::from(&label),
                    transducer_type: transducer_type.pop().unwrap(),
                    physical_dimension: physical_dimension.pop().unwrap(),
                    unit_minimum: unit_minimum.pop().unwrap(),
                    unit_maximum: unit_maximum.pop().unwrap(),
                    digital_minimum: digital_minimum.pop().unwrap(),
                    digital_maximum: digital_maximum.pop().unwrap(),
                    prefilter: prefilter.pop().unwrap(),
                    sample_rate: sample_rate.pop().unwrap(),
                });
        }

        dynamic_header.reserved = chunk_string(header.next(ct * 32), 32);

        dynamic_header
    }

    #[derive(Debug)]
    pub struct Header {
        pub file_metadata: FileMetadata,
        pub channel_metadata: ChannelMetadata,
    }

    // Some methods for convenience
    impl Header {
        pub fn get_channel_total(&self) -> usize {
            self.file_metadata.channel_total
        }

        pub fn get_header_size(&self) -> usize {
            self.file_metadata.header_size
        }

        pub fn get_record_size(&self) -> usize {
            self.file_metadata.records_total
        }

        pub fn get_duration(&self) -> usize {
            self.file_metadata.record_duration
        }
    }

    // Bdf Header section that is a fixed size
    #[derive(Debug)]
    pub struct FileMetadata {
        // nXX is next XX bytes
        // n(A)X is next A * X bytes, where A is a value given or unknown

        // first byte (255)
        pub special_bit: u8,
        
        // 2-8, BIOSEMI
        pub biosemi: String,
        
        // n80, Local subject id (ascii)
        pub subject_id: String,

        // n80, Local recording id (ascii)
        pub recording_id: String,

        // n8, starttime of recording (dd.mm.yy ascii)
        pub record_start_date: String,

        // n8, starttime of recording (hh.mm.ss ascii)
        pub record_start_time: String,

        // n8, number of bytes in header record
        pub header_size: usize,

        // n44, version data format ("24BIT" ascii)
        pub version: String,

        // n8, number of data records, can be "-1" if unknown (ascii)
        pub records_total: usize,

        // n8, duration of data record in seconds, (ascii number)
        pub record_duration: usize,

        // n4, number of channels (N) in data record, ("257" or "128", ascii)
        pub channel_total: usize,
    }

    pub type ChannelLabel = String;

    // Bdf header section that is dynamic in size based on total channels
    #[derive(Debug)]
    pub struct ChannelMetadata {

        pub channel_metadata: HashMap<ChannelLabel, Channel>,

        // n(N)32, reserved (ascii)
        pub reserved: Vec<String>,
    }

    impl ChannelMetadata {
        pub fn get_channels(&self) -> std::collections::hash_map::Values<ChannelLabel, Channel> {
            self.channel_metadata.values()
        }

        pub fn get_labels(&self) -> std::collections::hash_map::Keys<ChannelLabel, Channel> {
            self.channel_metadata.keys()
        }
    }

    // Each channel takes up 224 Bytes in the header when accounting for all the metadata
    #[derive(Debug)]
    pub struct Channel {
        // n(N)16, labels for channels, three letter labels "Fp1" (ascii)
        pub label: String,

        // n(N)80, transducer type, can be "active electrode", "respiration belt", etc. (ascii)
        pub transducer_type: String,

        // n(N)8, physical dimensions of channel, "uV" "Ohm", (ascii)
        pub physical_dimension: String,

        // n(N)8, phys. minimum in units of phy. dimension, number value (ascii)
        pub unit_minimum: i64,

        // n(N)8, phys. max in units of phy. dimension, number value (ascii)
        pub unit_maximum: i64,

        // n(N)8, digital minimum, number value (ascii)
        pub digital_minimum: i64,

        // n(N)8, digital maximum, number value (ascii)
        pub digital_maximum: i64,

        // n(N)80, prefiltering?, e.g. "HP:DC; LP:410", not specified if ascii
        pub prefilter: String,

        // n(N)8, number of samples in each data record, number value (ascii)
        pub sample_rate: u64,
    }

}

mod byte_reader {
    pub struct ByteReader<'a> {
        pub byte_data: &'a [u8],
        pub head_position: usize,
    }

    impl<'a> ByteReader<'a> {
        pub fn next(&mut self, delta: usize) -> &'a [u8] {
            if self.within_bounds(delta) {
                let start = self.head_position;
                let end = self.head_position + delta;
                self.head_position = end;

                return &self.byte_data[start..end];
            }

            &self.byte_data[..]
        }

        fn view_next(&self, delta: usize) -> &'a [u8] {
            if self.within_bounds(delta) {
                let start = self.head_position;
                let end = self.head_position + delta;
                return &self.byte_data[start..end];
            }

            &self.byte_data[..]
        }


        fn within_bounds(&self, delta: usize) -> bool {
            let length = self.byte_data.len();

            if self.head_position < length
                || self.head_position + delta < length
                || self.head_position+ delta > 0
            {
                return true
            }

            false
        }
    }
}

mod parse_helpers {

    use std::io::Cursor;

    use byteorder::{ReadBytesExt, LittleEndian};

    pub fn chunk_string(input: &[u8], size: usize) -> Vec<String> {
        input.chunks(size)
            .map(|c| String::from_utf8(c.to_vec()).unwrap())
            .map(|s| String::from(s.trim()))
            .collect::<Vec<String>>()
    }

    pub fn chunk_little_endian(input: &[u8], size: usize) -> Vec<i32> {
        input.chunks(size)
            .map(|c| Cursor::new(c).read_i24::<LittleEndian>().ok().unwrap())
            .collect::<Vec<i32>>()
    }

    pub fn chunk_i64(input: &[u8], size: usize) -> Vec<i64> {
        input.chunks(size)
            .map(|c| String::from_utf8(c.to_vec()).unwrap())
            .map(|s| String::from(s.trim()))
            .map(|s| s.parse::<i64>().unwrap())
            .collect::<Vec<i64>>()
    }

    pub fn chunk_u64(input: &[u8], size: usize) -> Vec<u64> {
        input.chunks(size)
            .map(|c| String::from_utf8(c.to_vec()).unwrap())
            .map(|s| String::from(s.trim()))
            .map(|s| s.parse::<u64>().unwrap())
            .collect::<Vec<u64>>()
    }

    pub fn string_from(bytes: &[u8]) -> String {
        let untrimmed_data = String::from_utf8(bytes.to_vec()).unwrap();
        String::from(untrimmed_data.trim())
    }
}
