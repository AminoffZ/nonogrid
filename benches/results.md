# Black-and-White puzzles

## Comparison with python solver

### Simple line solvable puzzles

| puzzle name | lines solved (python/rust) | python (PyPy), sec | rust (debug), sec | rust (release), sec | gain, times |
|-------------|----------------------------|-------------------:|------------------:|--------------------:|:-----------:|
| -b einstein | 1793/1679                  | 0.685..0.755       | 0.359..0.393      | 0.0114..0.0127      | 54..66      |
| -p 2992     | 2779/2634                  | 0.701..0.910       | 0.809..0.815      | 0.0214..0.0277      | 25..42      |
| -p 5933     | 3461/3192                  | 0.861..0.995       | 1.165..1.230      | 0.0313..0.0427      | 20..32      |
| -p 10564    | 2828/2914                  | 0.749..0.939       | 0.783..0.863      | 0.0257..0.0305      | 25..36      |
| -p 18845    | 1897/1366                  | 0.824..0.985       | 0.287..0.313      | 0.0082..0.0116      | 71..120     |
| -n 4438     | 3047                       | 1.061..1.216       | unimplemented     | unimplemented       | N/A         |
| -n 5114     | 5274                       | 1.940..2.137       | unimplemented     | unimplemented       | N/A         |
| -n 5178     | 3421                       | 1.146..1.380       | unimplemented     | unimplemented       | N/A         |
| -n 19043    | 4608                       | 1.043..1.286       | unimplemented     | unimplemented       | N/A         |


### Probing solver

| puzzle name | contradictions (python/rust) | python (PyPy), sec | rust (debug), sec | rust (release), sec | gain, times |
|-------------|------------------------------|-------------------:|------------------:|--------------------:|:-----------:|
| -b MLP      | 429/?                        | 3.200..4.617       | 3.404..3.982      | 0.122..0.162        | 19..38      |
| -p 2040     | 204/?                        | 1.922..2.349       | 2.384..3.500      | 0.095..0.124        | 15..25      |



## Hardest backtracking puzzles

### Sqrt strategy, single cache, more than 30 seconds

| puzzle_id | solve time, sec | depth, levels | solutions | final rate |
|-----------|----------------:|--------------:|:---------:|-----------:|
| _3867_    | 296.38          | 21            | 2         | 0.8710     |
| **8098**  | 36.51           | 8             | 1         | 1          |
| **9892**  | 259.68          | 22            | 2         | 0.4385     |
| **12548** | +               | 40            | 0         | 0.1856     |
| 13480     | +               | 38            | 0         | 0.1263     | FIXME: why starting rate r=12.68 in Python?
| 16900     | 297.03          | 30            | 2         | 0.4712     | FIXME: why starting rate r=48.06 in Python?
| **18297** | 885.24          | 14            | 2         | 0.1349     |
| **22336** | +               | 12            | 0         | 0.4313     |
| 25385     | 31.23           | 36            | 2         | 0.2656     | FIXME: Sometimes it solves for hours.
| 25820     | +               | 31            | 0         | 0.0552     |
| 26520     | +               | 43            | 0         | 0.0836     |
| 27174     | 80.36           | 8             | 1         | 1          |
| 30509     | 36.95           | 98            | 2         | 0          |
| 30681     | 157.24          | 23            | 2         | 0.5464     |

### Max strategy, single cache

| puzzle_id | solve time, sec | depth, levels | solutions | final rate |
|-----------|----------------:|--------------:|:---------:|-----------:|
| _3867_    | 0.22            | 14            | 2         | 0.8710     |
| **8098**  | 46.56           | 7             | 1         | 1          |
| **9892**  | +               | 19            | 2         | 0.3330     |
| **12548** | +               | 18            | 0         | 0.1856     |
| 13480     | 1.39            | 16            | 2         | 0.1263     |
| 16900     | +               | 33            | 0         | 0.4866     |
| **18297** | +               | 15            | 0         | 0.1409     |
| **22336** | +               | 7             | 0         | 0.4263     |
| 25385     | 791.80          | 18            | 2         | 0.2664     |
| 25820     | +               | 20            | 0         | 0.0558     |
| 26520     | +               | 27            | 0         | 0.0836     |


`+` means the solving time exceeds 3600 seconds and was interrupted

**Bold** puzzles are from http://webpbn.com/survey/ (_italic_ puzzles are mentioned there too).