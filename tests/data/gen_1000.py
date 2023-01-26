#!/usr/bin/env python3

# example given to chatGPT
# { "path": "/actors/1", "datetime": "2023-01-11T23:17:57+0000",
# "values": {"1": 10.46, "2": 102, "3": 3.004} }

# performance notes:
#
# 1 day 100 devices 10 per sec took 10 minutes to ingest 1.2 million recs.
# 1 day 100 devices 10 per sec took 11 minutes to ingest 1.2 million recs.
# - will try next to run w/o any println output actor
# 1 day 100 devices 10 per sec took 9 minutes to ingest 1.2 million recs.
# - will try next to run w/o any persist actor
# 1 day 100 devices 10 per sec took 1 minutes to ingest 1.2 million recs.
#

import json
import random
import pytz
from datetime import datetime, timedelta
import argparse

parser = argparse.ArgumentParser()

parser.add_argument("observations_per_minute", type=int,
                    help="Number of observations per minute per device.")
parser.add_argument("number_of_devices", type=int, help="Number of devices.")
parser.add_argument("number_of_days", type=int,
                    help="Number of days to simulate.")
parser.add_argument("start_date",
                    type=str, help="Start date in format yyyy-mm-dd.")

# Parse the command-line arguments
args = parser.parse_args()

# Convert the start_date string to a datetime object
start_date = datetime.strptime(args.start_date, "%Y-%m-%d")
start_datetime = datetime(start_date.year,
                          start_date.month,
                          start_date.day, 0, 0, 0, tzinfo=pytz.UTC)

# Calculate the number of observations to generate
minutes_per_day = 24 * 60
total_minutes = minutes_per_day * args.number_of_days

# Generate observations for the number of devices specified
device_ids = list(range(1, args.number_of_devices + 1))

for i in range(total_minutes * args.observations_per_minute):
    # Shuffle the list of device IDs
    random.shuffle(device_ids)
    for device_id in device_ids:
        # Generate a random number of milliseconds
        random_milliseconds = random.randint(-500, 500)
        # Increment datetime by 1 minute plus random milliseconds
        current_datetime = start_datetime + timedelta(
                minutes=i / args.observations_per_minute,
                milliseconds=random_milliseconds)
        # Randomize values for keys 1, 2, and 3
        values = {"1": round(random.uniform(0, 100), 2),
                  "2": random.randint(0, 200),
                  "3": round(random.uniform(0, 10), 2)}
        # Generate random number to decide whether to suppress this observation
        suppress_observation = random.randint(1, 10)
        if suppress_observation != 10:
            # Create record
            record = {"path": f"/actors/{device_id}",
                      "datetime": current_datetime.isoformat(),
                      "values": values}
            # Print record as JSON
            print(json.dumps(record))
