# reminder-rs
Reminder Bot for Discord, now in Rust.
Old Python version: https://github.com/reminder-bot/bot

## What is this?
This is a rewrite of Reminder Bot in Rust, using Serenity and SQLx amongst other libraries.

## Why is this?
Serenity and Rust are proving wonders for SoundFX. This is all in an effort to reduce resource consumption and improve performance.

## How do I use it?
You'll need rustc and cargo for compilation. To run, you'll need Python 3 still (due to no suitable replacement for dateparser in Rust)

### Compiling
Reminder Bot can be built by running `cargo build --release` in the top level directory. It is necessary to create a folder called 'assets' containing an image file with its name specified in the environment as `WEBHOOK_AVATAR`, of dimensions 128x128px to be used as the webhook avatar.

### Setting up Python
Reminder Bot by default looks for a venv within it's working directory to run Python out of. To set up a venv, install `python3-venv` and run `python3 -m venv venv`. Then, run `source venv/bin/activate` to activate the venv, and do `pip install dateparser` to install the required library

### Environment Variables
Reminder Bot reads a number of environment variables. Some are essential, and others have hardcoded fallbacks. Environment variables can be loaded from a .env file in the working directory.

__Required Variables__
* `DATABASE_URL` - the URL of your MySQL database (`mysql://user[:password]@domain/database`)
* `DISCORD_TOKEN` - your application's bot user's authorization token

__Other Variables__
* `LOCAL_TIMEZONE` - default `UTC`, necessary for calculations in the natural language processor
* `DEFAULT_PREFIX` - default `$`, used for the default prefix on new guilds
* `SUBSCRIPTION_ROLES` - default `None`, accepts a list of Discord role IDs that are given to subscribed users
* `CNC_GUILD` - default `None`, accepts a single Discord guild ID for the server that the subscription roles belong to
* `IGNORE_BOTS` - default `1`, if `1`, Reminder Bot will ignore all other bots
* `PYTHON_LOCATION` - default `venv/bin/python3`. Can be changed if your Python executable is located somewhere else
* `LOCAL_LANGUAGE` - default `EN`. Specifies the string set to fall back to if a string cannot be found (and to be used with new users)
* `THEME_COLOR` - default `8fb677`. Specifies the hex value of the color to use on info message embeds 
* `WEBHOOK_AVATAR` - default `None`, accepts the path to an image file to be used as the avatar when creating webhooks. **IMPORTANT: image file must be 128x128 or smaller in size**
* `CASE_INSENSITIVE` - default `1`, if `1`, commands will be treated with case insensitivity (so both `$help` and `$HELP` will work)
