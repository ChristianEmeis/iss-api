# ISS Api

### Features

- "/isspos" Get the current location and hight of the International Space Station calculated from the TLE data from the official NASA TLE Api
- "/isspath" Get the predicted path that the ISS will follow in the next 92 Minutes (≈ orbital period)
- There is a rate limit of 5 Requests per minute (both endpoints combined)

### ISS Position Endpoint
- "localhost:3000/isspos"
#### Example Response: 

```json5
{
    "lat": -45.81572038607639, // Current latitude of the ISS
    "lon": 57.84392412960222, // Current longitude of the ISS
    "height": 421.2752121758658, // Current height of the ISS in kilometer
    "timestamp": 1661364355 // Timestamp when the data was calculated
}
```

### ISS Path Endpoint
- "localhost:3000/isspath"
- *Note: Due to this calculation beeing relatively "resource costly", the path is beeing cached for 30 seconds*
#### Example Response: 

```json5
{
    "time": "2022-08-24T18:10:39.758711357Z", // Time when the data was calculated
    "path": [   // 92 Path Objects with a difference of 1 minute each containing the latitude and longitude at the time
        {
        "lat": -35.4479413679225,
        "lon": 76.63319147852752
        },
        {
        "lat": -32.87705836852733,
        "lon": 79.87781247484233
        },
        ... // + 90 times
    ]
}
```

### Disclaimer

This publication is provided as is without any express or implied warranties.
While every effort has been taken to ensure the accuracy of the information contained in this publication, the authors/maintainers/contributors assume no responsibility for errors or omissions, or for damages resulting from the use of the information contained herein.
