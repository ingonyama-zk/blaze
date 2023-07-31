import csv
import re

log_file = "test.log"
output_file = "output.csv"

# Regular expression patterns to match timestamps and values
timestamp_pattern = r"\[(.*?)\]"
value_pattern = r"value:\s+(.*?)$"

data = []

# Read the log file
with open(log_file, "r") as file:
    lines = file.readlines()

    # Iterate over each line in the log file
    for i in range(len(lines)):
        line = lines[i]
        if "===" in line and "api values" in line:
            values = []
            for j in range(i+1, len(lines)):
                inner_line = lines[j]
                if "value:" in inner_line:
                    value_match = re.search(value_pattern, inner_line)
                    if value_match:
                        value = value_match.group(1)
                        values.append(value)
                elif "===" in inner_line and "api values" in inner_line:
                    break
            if len(values) == 3:
                timestamp_match = re.search(timestamp_pattern, line)
                if timestamp_match:
                    timestamp = timestamp_match.group(1)
                    data.append([timestamp] + values)

# Write the data to a CSV file
with open(output_file, "w", newline="") as csv_file:
    writer = csv.writer(csv_file)
    writer.writerow(["Timestamp", "ADDR_HIF2CPU_C_NOF_ELEMENTS_PENDING_ON_DMA_FIFO", "ADDR_HIF2CPU_C_NOF_RESULTS_PENDING_ON_DMA_FIFO", "ADDR_HIF2CPU_C_MAX_RECORDED_PENDING_RESULTS"])  # Write header row

    # Write data rows
    writer.writerows(data)

print("CSV file created successfully.")