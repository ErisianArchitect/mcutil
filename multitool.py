"""
This script is used to do multiple things.
Things it will do:
    Create new journal entries for the specified date.
"""
__date_format = "{day_of_week}, {month} {day}, {year}"
journal_entry_template = """# {date} : {title}
### What I'm working on:
> 
### Entry:

***
"""
from typing import *
from datetime import datetime
from pathlib import Path
import calendar
from calendar import Calendar

__journal_path = Path(".\\journal\\")

class Weekday:
    __slots__ = ('name')
    def __init__(self, name: str):
        self.name = name.capitalize()
    @property
    def short(self)->str:
        return self.name[:3]


_us_weekday = [
    Weekday('Sunday'),
    Weekday('Monday'),
    Weekday('Tuesday'),
    Weekday('Wednesday'),
    Weekday('Thursday'),
    Weekday('Friday'),
    Weekday('Saturday'),
]

_world_weekday = [
    Weekday('Monday'),
    Weekday('Tuesday'),
    Weekday('Wednesday'),
    Weekday('Thursday'),
    Weekday('Friday'),
    Weekday('Saturday'),
    Weekday('Sunday'),
]

def get_date_date_string(date: datetime = None)->str:
    date = date or datetime.now()
    day = date.day
    day_suffix = get_ordinal_suffix(day)
    day = f'{day}{day_suffix}'
    return __date_format.format(
        day_of_week = date.strftime("%A"),
        month = date.strftime("%B"),
        day = day,
        year = date.year,
    )

def get_entry_path(date: datetime = None,*,extension='.md')->Path:
    if date is None:
        date = datetime.now()
    year_path = __journal_path.joinpath(str(date.year))
    month_path = year_path.joinpath(date.strftime('%B'))
    ord_suffix = get_ordinal_suffix(date.day)
    # Fix the extension.
    match extension:
        case None:
            extension = ''
        # If the extension doesn't start with '.', it needs to be prepended.
        case str(ext) if ext[:1] != '.':
            extension = f'.{ext}'
    file_name = f'{date.day}{ord_suffix}{extension}'
    day_path = month_path.joinpath(file_name)

def get_ordinal_suffix(n: int)->str:
    """Gets the ordinal suffix of a number.
    
    This is also called the ordinal indicator.
    It is the `st` on `1st`, the `nd` on `2nd`, and the `rd` on `3rd`."""
    match n:
        case 11:
            return 'th'
        case 12:
            return 'th'
        case 13:
            return 'th'
        case n:
            match n % 10:
                case 1:
                    return 'st'
                case 2:
                    return 'nd'
                case 3:
                    return 'rd'
                case _:
                    return 'th'

def create_entry_string()->str:
    now = datetime.now()
    day = now.day
    dom_suffix = get_ordinal_suffix(day)
    day = f'{day}{dom_suffix}'
    format_args = dict(
        day_of_week = now.strftime("%A"),
        month = now.strftime("%B"),
        day = day,
        year = now.year,
    )
    return journal_entry_template.format(**format_args)

def format_date_table_day(day: int)->str:
    fmt = " {:>4} "
    day_ord = get_ordinal_suffix(day)
    return fmt.format(f'{day}{day_ord}')

def format_week(days: Iterable,*,formatter = None)->str:
    """
    Returns a line for a table for the following week.
    The iterable should return 7 elements.
    """
    def day_formatter(day: int | None, formatter = formatter)->str:
        match day:
            case int(day):
                return format_date_table_day(day)
            case str(day):
                return ' {:>4} '.format(day[:4])
            case _:
                return '      '
    return '|{}|'.format('|'.join(map(day_formatter,days)))

def _adjust_weekday(weekday: int)->int:
    return (weekday + 1) % 7

def format_month(year: int, month: int, formatter: Callable = None)->str:
    # With the week day, we know where to start the first week of the month.
    year = year or datetime.now().year
    month_start, day_count = calendar.monthrange(year, month)
    month_start = _adjust_weekday(month_start)
    weeks = (day_count // 7) + (day_count % 7 != 0)
    first_week = format_week([*([None]*month_start), *range(1, 8-month_start)])
    next_week_start = 8 - month_start
    next_week_end = next_week_start + 7
    print(next_week_start)
