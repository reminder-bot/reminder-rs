import dateparser
import sys
import pytz
from datetime import datetime

dt = dateparser.parse(sys.argv[1], settings={
    'TIMEZONE': sys.argv[2],
    'TO_TIMEZONE': sys.argv[3],
    'RELATIVE_BASE': datetime.now(pytz.timezone(sys.argv[2])).replace(tzinfo=None),
    'PREFER_DATES_FROM': 'future',
})

print(dt.timestamp() if dt is not None else -1)
