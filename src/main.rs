use calamine::{open_workbook, Reader, Xlsx};
use regex::Regex;
use std::{fs, io, path::Path, process::exit};

const FAILURES_FOLDER: &str = "./failures";

fn press_to_exit(exit_code: i32) -> ! {
    println!("Press enter to exit");
    let mut buffer = String::new();
    let _ = io::stdin().read_line(&mut buffer);
    exit(exit_code);
}

fn rem_last(value: &String) -> String {
    let mut chars = value.chars();
    chars.next_back();
    return String::from(value);
}

fn file_name_to_station_name(file_name: &String) -> Result<String, String> {
    let re = match Regex::new(r"^error_failure[A-Z]{3}[1-9]{1}-") {
        Ok(exp) => exp,
        Err(e) => {
            return Err(e.to_string());
        }
    };

    let file_name_match = match re.find(file_name.as_str()) {
        Some(m) => m,
        None => {
            return Err("Can't find station name in file name".to_string());
        }
    };

    let start = "error_failure".len();
    let end = &file_name_match.as_str().len() - 1;
    let station_name = &file_name_match.as_str()[start..end];

    Ok(station_name.to_string())
}

fn main() {
    let dir = match fs::read_dir(FAILURES_FOLDER) {
        Ok(d) => d,
        Err(_) => {
            println!("Error opening the 'failures' folder. Please make sure this tool and the failures folder are in the same directory.");
            press_to_exit(1);
        }
    };

    let output_file = "output.csv";
    let mut output_data = String::new();
    let mut total_file_count = 0;
    let mut failed_file_count = 0;
    let mut failed_dsp_count = 0;
    for item in dir
        .filter_map(|x| {
            total_file_count += 1;
            x.ok()
        })
        .filter(|x| {
            x.file_name()
                .to_str()
                .is_some_and(|s| s.starts_with("error_failure"))
        })
    {
        failed_file_count += 1;
        let file_path = Path::new(FAILURES_FOLDER).join(item.file_name());
        let mut workbook: Xlsx<_> = match open_workbook(file_path) {
            Ok(s) => s,
            Err(e) => {
                println!("{:?}", e);
                continue;
            }
        };

        if let Some(Ok(sheet)) = workbook.worksheet_range("Nursery Routes") {
            let mut sheet_rows = sheet.rows();
            let headers_row = match sheet_rows.next() {
                Some(r) => r,
                None => continue,
            };

            let result_column_index: u32 = match headers_row
                .iter()
                .enumerate()
                .find(|(_i, h)| h.to_string() == String::from("result"))
                .map(|(u, _)| u.try_into().ok())
                .flatten()
            {
                Some(index) => index,
                None => {
                    println!(
                        "{:?} - Skipping. Couldn't find the results header in the file",
                        item.file_name()
                    );
                    continue;
                }
            };

            let failure_rows =
                sheet_rows.filter(|row| match row.get(result_column_index as usize) {
                    Some(r) => r.to_string() == String::from("Failure"),
                    None => false,
                });

            if output_data.len() == 0 {
                output_data += "filename,station_name,";
                headers_row
                    .iter()
                    .for_each(|h| output_data += &format!("{},", h));
                output_data = rem_last(&output_data);
                output_data += "\n";
            }

            failure_rows.for_each(|r| {
                failed_dsp_count += 1;
                output_data += &format!("{},", item.file_name().to_string_lossy());
                let station_name = match file_name_to_station_name(&format!(
                    "{},",
                    item.file_name().to_string_lossy()
                )) {
                    Ok(name) => name,
                    Err(_) => {
                        println!(
                            "Couldn't find station name in file '{}'. Using 'unknown' in output.",
                            item.file_name().to_string_lossy()
                        );
                        "unknown".to_string()
                    }
                };
                output_data += &format!("{:?},", station_name);
                r.iter().for_each(|v| {
                    output_data += &format!("{},", format!("{}", v).replace(",", ""))
                });
                output_data = rem_last(&output_data);
                output_data += &String::from("\n");
            });
        }
    }

    println!("Found {} total files in failures folder", total_file_count);
    println!("Found {} failed nursery files", failed_file_count);
    println!("Found {} total failed DPSs", failed_dsp_count);

    match fs::write(output_file, output_data) {
        Ok(_) => {
            println!("Output saved to {}", output_file);
            press_to_exit(0);
        }
        Err(e) => {
            println!("{}", e);
            press_to_exit(1)
        }
    }
}
