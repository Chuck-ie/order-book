import csv
import re

def parse_criterion_log(input_file_path, output_csv_path):
    path_regex = re.compile(
        r"Level Scaling/([^/]+)/([^/]+)/([^/]+)/levels_(\d+)/orders_(\d+)"
    )
    thrpt_regex = re.compile(r"thrpt:\s+\[([^\]]+)\]")

    unique_results = {}
    current_entry = None
    in_change_section = False

    with open(input_file_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()

            if "change:" in line:
                in_change_section = True
                continue

            path_match = path_regex.search(line)
            if path_match:
                in_change_section = False

                order_type = path_match.group(1).strip()
                if order_type.endswith(" Orders"):
                    order_type = order_type[: -len(" Orders")]

                current_entry = {
                    "engine": path_match.group(3).strip(),
                    "command_type": order_type,
                    "order_strategy": path_match.group(2).strip(),
                    "levels": int(path_match.group(4)),
                    "orders_per_level": int(path_match.group(5)),
                }
                continue

            if current_entry and not in_change_section and "thrpt:" in line:
                thrpt_match = thrpt_regex.search(line)
                if thrpt_match:
                    parts = thrpt_match.group(1).split()

                    if len(parts) >= 4:
                        try:
                            mid_value = float(parts[2])
                            unit = parts[3]

                            if "Kelem" in unit:
                                throughput_million = mid_value / 1000.0
                            elif "Melem" in unit:
                                throughput_million = mid_value
                            elif "Gelem" in unit:
                                throughput_million = mid_value * 1000.0
                            else:
                                throughput_million = mid_value / 1000000.0

                            current_entry["m_orders_per_second"] = round(
                                throughput_million, 5
                            )

                            key = (
                                current_entry["engine"],
                                current_entry["command_type"],
                                current_entry["order_strategy"],
                                current_entry["levels"],
                                current_entry["orders_per_level"],
                            )

                            unique_results[key] = current_entry

                        except ValueError:
                            pass

                current_entry = None

    headers = [
        "engine",
        "command_type",
        "order_strategy",
        "levels",
        "orders_per_level",
        "m_orders_per_second",
    ]

    with open(output_csv_path, "w", newline="", encoding="utf-8") as csv_f:
        writer = csv.DictWriter(csv_f, fieldnames=headers)
        writer.writeheader()
        writer.writerows(unique_results.values())

    print(
        f"Success: Parsed {len(unique_results)} unique entries into '{output_csv_path}'."
    )

if __name__ == "__main__":
    parse_criterion_log("benches/results/criterion_results.txt", "benches/results/criterion_results.csv")
