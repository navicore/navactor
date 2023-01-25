#!/usr/bin/env python3

# example given to chatGPT
# { "path": "/actors/1", "datetime": "2023-01-11T23:17:57+0000",
# "values": {"1": 10.46, "2": 102, "3": 3.004} }

import json
import random
import pytz
from datetime import datetime, timedelta
import argparse

# Initialize starting datetime
start_datetime = datetime(2023, 1, 3, 0, 0, 0, tzinfo=pytz.UTC)

# Create an ArgumentParser object
parser = argparse.ArgumentParser()

# Add the "observations_per_minute" and "number_of_devices" arguments
parser.add_argument("observations_per_minute", type=int, help="Number of observations per minute per device.")
parser.add_argument("number_of_devices", type=int, help="Number of devices.")

# Parse the command-line arguments
args = parser.parse_args()

# Calculate the number of observations to generate
minutes_per_day = 24 * 60

# Generate observations for the number of devices specified
device_ids = list(range(1, args.number_of_devices + 1))

for i in range(minutes_per_day * args.observations_per_minute):
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
