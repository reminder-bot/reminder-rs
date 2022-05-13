# reminder-rs
Reminder Bot for Discord.

## How do I use it?
I offer a hosted version of the bot. You can invite it with: **https://invite.reminder-bot.com**. The catch is that repeating 
reminders are paid on the hosted version of the bot. Keep reading if you want to host it yourself.

You'll need rustc and cargo for compilation. To run, you'll need Python 3 still (due to no suitable replacement for dateparser in Rust)

### Compiling
Install build requirements: 
`sudo apt install gcc gcc-multilib cmake libssl-dev build-essential`

Install Rust from https://rustup.rs

Reminder Bot can then be built by running `cargo build --release` in the top level directory. It is necessary to create a 
folder called 'assets' containing an image file with its name specified in the environment as `WEBHOOK_AVATAR`, of 
dimensions 128x128px to be used as the webhook avatar.

#### Compilation environment variables
These environment variables must be provided when compiling the bot
* `DATABASE_URL` - the URL of your MySQL database (`mysql://user[:password]@domain/database`)
* `WEBHOOK_AVATAR` - accepts the name of an image file located in `$CARGO_MANIFEST_DIR/assets/` to be used as the avatar when creating webhooks. **IMPORTANT: image file must be 128x128 or smaller in size**

### Setting up Python
Reminder Bot by default looks for a venv within it's working directory to run Python out of. To set up a venv, install `python3-venv` and run `python3 -m venv venv`. Then, run `source venv/bin/activate` to activate the venv, and do `pip install dateparser` to install the required library

### Environment Variables
Reminder Bot reads a number of environment variables. Some are essential, and others have hardcoded fallbacks. Environment variables can be loaded from a .env file in the working directory.

__Required Variables__
* `DATABASE_URL` - the URL of your MySQL database (`mysql://user[:password]@domain/database`)
* `DISCORD_TOKEN` - your application's bot user's authorization token

__Other Variables__
* `MIN_INTERVAL` - default `600`, defines the shortest interval the bot should accept
* `LOCAL_TIMEZONE` - default `UTC`, necessary for calculations in the natural language processor
* `SUBSCRIPTION_ROLES` - default `None`, accepts a list of Discord role IDs that are given to subscribed users
* `CNC_GUILD` - default `None`, accepts a single Discord guild ID for the server that the subscription roles belong to
* `PYTHON_LOCATION` - default `venv/bin/python3`. Can be changed if your Python executable is located somewhere else
* `THEME_COLOR` - default `8fb677`. Specifies the hex value of the color to use on info message embeds 
* `DM_ENABLED` - default `1`, if `1`, Reminder Bot will respond to direct messages

### Todo List

* Convert aliases to macros
