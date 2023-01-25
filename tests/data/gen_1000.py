#!/usr/bin/env python3

# example given to chatGPT
# { "path": "/actors/1", "datetime": "2023-01-11T23:17:57+0000",
# "values": {"1": 10.46, "2": 102, "3": 3.004} }

import json
import random
import pytz
from datetime import datetime, timedelta

# Initialize starting datetime
start_datetime = datetime(2023, 1, 3, 0, 0, 0, tzinfo=pytz.UTC)

# Calculate the number of observations to generate
observations_per_second = 10
observation_rate = 5
seconds_per_day = 24 * 60 * 60
observations_per_day = observations_per_second * observation_rate

# Generate observations for 10 devices
device_ids = list(range(1, 11))

for i in range(seconds_per_day * observations_per_second):
    # Shuffle the list of device IDs
    random.shuffle(device_ids)
    for device_id in device_ids:
        # Generate a random number of milliseconds
        random_milliseconds = random.randint(-500, 500)
        # Increment datetime by 1 second plus random milliseconds
        current_datetime = start_datetime + timedelta(seconds=i / observations_per_second, milliseconds=random_milliseconds)
        
        # Randomize values for keys 1, 2, and 3
        values = {"1": round(random.uniform(0, 100), 2), "2": random.randint(0, 200), "3": round(random.uniform(0, 10), 2)}
        # Create record
        record = {"path": f"/actors/{device_id}", "datetime": current_datetime.isoformat(), "values": values}

        # Print record as JSON
        print(json.dumps(record))
