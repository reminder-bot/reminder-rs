let colorPicker = new iro.ColorPicker("#colorpicker");
let $discordFrame;
const $loader = document.querySelector("#loader");
const $colorPickerModal = document.querySelector("div#pickColorModal");
const $colorPickerInput = $colorPickerModal.querySelector("input");

let timezone = luxon.DateTime.now().zone.name;
const browserTimezone = luxon.DateTime.now().zone.name;
let botTimezone = "UTC";

let channels;
let roles;

const remindersLoadedEvent = new Event("remindersLoaded");

function colorToInt(r, g, b) {
    return (r << 16) + (g << 8) + b;
}

function resize_textareas() {
    document.querySelectorAll("textarea.autoresize").forEach((element) => {
        element.style.height = "";
        element.style.height = element.scrollHeight + 3 + "px";

        element.addEventListener("input", () => {
            element.style.height = "";
            element.style.height = element.scrollHeight + 3 + "px";
        });
    });
}

function switch_pane(selector) {
    document.querySelectorAll("aside a").forEach((el) => {
        el.classList.remove("is-active");
    });
    document.querySelectorAll("div.is-main-content > section").forEach((el) => {
        el.classList.add("is-hidden");
    });

    document.getElementById(selector).classList.remove("is-hidden");

    resize_textareas();
}

function update_select(sel) {
    if (sel.selectedOptions[0].dataset["webhookAvatar"]) {
        sel
            .closest("div.reminderContent")
            .querySelector("img.discord-avatar").src =
            sel.selectedOptions[0].dataset["webhookAvatar"];
    } else {
        sel
            .closest("div.reminderContent")
            .querySelector("img.discord-avatar").src = "";
    }
    if (sel.selectedOptions[0].dataset["webhookName"]) {
        sel
            .closest("div.reminderContent")
            .querySelector("input.discord-username").value =
            sel.selectedOptions[0].dataset["webhookName"];
    } else {
        sel
            .closest("div.reminderContent")
            .querySelector("input.discord-username").value = "";
    }
}

function reset_guild_pane() {
    document
        .querySelectorAll("select.channel-selector option")
        .forEach((opt) => opt.remove());
}

function fetch_roles(guild_id) {
    fetch(`/dashboard/api/guild/${guild_id}/roles`)
        .then((response) => response.json())
        .then((data) => {
            if (data.error) {
                show_error(data.error);
            } else {
                roles = data;
            }
        });
}

async function fetch_reminders(guild_id) {
    // fetch dm reminders instead
    if (guild_id === undefined) {
        const $reminderBox = document.querySelector("div#personalReminders");

        // reset div contents
        $reminderBox.innerHTML = "";

        // fetch reminders
        await fetch("dashboard/api/user/reminders")
            .then((response) => response.json())
            .then((data) => {
                if (data.error) {
                    show_error(data.error);
                } else {
                    const $template = document.querySelector(
                        "template#personalReminder"
                    );

                    for (let reminder of data) {
                        let newFrame = $template.content.cloneNode(true);

                        for (let prop in reminder) {
                            if (
                                reminder.hasOwnProperty(prop) &&
                                reminder[prop] !== null
                            ) {
                                let $input = newFrame.querySelector(
                                    `*[name="${prop}"]`
                                );
                                let $image = newFrame.querySelector(
                                    `img.${prop}`
                                );

                                if ($input !== null) {
                                    $input.value = reminder[prop];
                                } else if ($image !== null) {
                                    $image.src = reminder[prop];
                                }
                            }
                        }

                        $reminderBox.append(newFrame);
                    }
                }
            });
    } else {
        const $reminderBox = document.querySelector("div#guildReminders");

        // reset div contents
        $reminderBox.innerHTML = "";

        // fetch reminders
        await fetch(`dashboard/api/guild/${guild_id}/reminders`)
            .then((response) => response.json())
            .then((data) => {
                if (data.error) {
                    show_error(data.error);
                } else {
                    console.log(data);

                    const $template = document.querySelector(
                        "template#guildReminder"
                    );

                    for (let reminder of data) {
                        let newFrame = $template.content.cloneNode(true);

                        newFrame.querySelector(".reminderContent").dataset.uid =
                            reminder["uid"];

                        // populate channels
                        set_channels(
                            newFrame.querySelector("select.channel-selector")
                        );

                        // populate majority of items
                        for (let prop in reminder) {
                            if (
                                reminder.hasOwnProperty(prop) &&
                                reminder[prop] !== null
                            ) {
                                let $input = newFrame.querySelector(
                                    `*[name="${prop}"]`
                                );
                                let $image = newFrame.querySelector(
                                    `img.${prop}`
                                );

                                if ($input !== null) {
                                    $input.value = reminder[prop];
                                } else if ($image !== null) {
                                    $image.src = reminder[prop];
                                }
                            }
                        }

                        let guild = document.querySelector(
                            ".guildList a.is-active"
                        ).dataset["guild"];

                        let $enableBtn =
                            newFrame.querySelector(".disable-enable");

                        $enableBtn.textContent = reminder["enabled"]
                            ? "Disable"
                            : "Enable";
                        $enableBtn.addEventListener("click", () => {
                            fetch(`/dashboard/api/guild/${guild}/reminders`, {
                                method: "PATCH",
                                body: JSON.stringify({
                                    uid: reminder["uid"],
                                    enabled: false,
                                }),
                            });
                        });

                        let timeInput =
                            newFrame.querySelector('input[name="time"]');
                        let localTime = luxon.DateTime.fromISO(
                            reminder["utc_time"]
                        ).setZone(timezone);
                        timeInput.value = localTime.toFormat(
                            "yyyy-LL-dd'T'HH:mm:ss"
                        );

                        $reminderBox.appendChild(newFrame);
                    }
                }
            });
    }

    document.dispatchEvent(remindersLoadedEvent);
    register_interval_hide();
}

function show_error(error) {
    document
        .getElementById("errors")
        .querySelector("span.error-message").textContent = error;
    document.getElementById("errors").classList.add("is-active");

    window.setTimeout(() => {
        document.getElementById("errors").classList.remove("is-active");
    }, 5000);
}

function update_times() {
    document.querySelectorAll("span.set-timezone").forEach((element) => {
        element.textContent = timezone;
    });
    document.querySelectorAll("span.set-time").forEach((element) => {
        element.textContent = luxon.DateTime.now()
            .setZone(timezone)
            .toFormat("HH:mm");
    });
    document.querySelectorAll("span.browser-timezone").forEach((element) => {
        element.textContent = browserTimezone;
    });
    document.querySelectorAll("span.browser-time").forEach((element) => {
        element.textContent = luxon.DateTime.now().toFormat("HH:mm");
    });
    document.querySelectorAll("span.bot-timezone").forEach((element) => {
        element.textContent = botTimezone;
    });
    document.querySelectorAll("span.bot-time").forEach((element) => {
        element.textContent = luxon.DateTime.now()
            .setZone(botTimezone)
            .toFormat("HH:mm");
    });
}

window.setInterval(() => {
    update_times();
}, 30000);

document.getElementById("set-bot-timezone").addEventListener("click", () => {
    timezone = botTimezone;
    update_times();
});
document
    .getElementById("set-browser-timezone")
    .addEventListener("click", () => {
        timezone = browserTimezone;
        update_times();
    });
document.getElementById("update-bot-timezone").addEventListener("click", () => {
    timezone = browserTimezone;
    fetch("/dashboard/api/user", {
        method: "PATCH",
        headers: {
            Accept: "application/json",
            "Content-Type": "application/json",
        },
        body: JSON.stringify({ timezone: timezone }),
    })
        .then((response) => response.json())
        .then((data) => {
            if (data.error) {
                show_error(data.error);
            } else {
                botTimezone = browserTimezone;
                update_times();
            }
        });
});

$colorPickerInput.value = colorPicker.color.hexString;

$colorPickerInput.addEventListener("input", () => {
    if (/^#[0-9a-fA-F]{6}$/.test($colorPickerInput.value) === true) {
        colorPicker.color.hexString = $colorPickerInput.value;
    }
});

colorPicker.on("color:change", function (color) {
    $colorPickerInput.value = color.hexString;
});

document.querySelectorAll(".discord-embed").forEach((element) => {
    element.addEventListener("click", (e) => {
        if (
            e.offsetX <
            parseInt(window.getComputedStyle(element).borderLeftWidth)
        ) {
            $discordFrame = element;
            $colorPickerModal.classList.toggle("is-active");
            colorPicker.color.rgbString =
                window.getComputedStyle($discordFrame).borderLeftColor;
        }
    });
});

document.querySelectorAll(".set-color").forEach((element) => {
    element.addEventListener("click", (e) => {
        e.preventDefault();

        $discordFrame = element
            .closest("div.reminderContent")
            .querySelector("div.discord-embed");
        $colorPickerModal.classList.toggle("is-active");
        colorPicker.color.rgbString =
            window.getComputedStyle($discordFrame).borderLeftColor;
    });
});

$colorPickerModal
    .querySelector("button.is-success")
    .addEventListener("click", () => {
        $discordFrame.style.borderLeftColor = colorPicker.color.rgbString;

        $colorPickerModal.classList.remove("is-active");
    });

document.querySelectorAll("a.show-modal").forEach((element) => {
    element.addEventListener("click", (e) => {
        e.preventDefault();
        document
            .getElementById(element.dataset["modal"])
            .classList.toggle("is-active");
    });
});

document.addEventListener("DOMContentLoaded", () => {
    $loader.classList.remove("is-hidden");

    document.querySelectorAll(".navbar-burger").forEach((el) => {
        el.addEventListener("click", () => {
            const target = el.dataset.target;
            const $target = document.getElementById(target);

            el.classList.toggle("is-active");
            $target.classList.toggle("is-active");
        });
    });

    fetch("/dashboard/api/user")
        .then((response) => response.json())
        .then((data) => {
            if (data.error) {
                show_error(data.error);
            } else {
                document
                    .querySelectorAll("a.switch-pane")
                    .forEach((element) => {
                        element.innerHTML = element.innerHTML.replace(
                            "%username%",
                            data.name
                        );

                        element.addEventListener("click", (e) => {
                            e.preventDefault();

                            switch_pane(element.dataset["pane"]);

                            element.classList.add("is-active");

                            document
                                .querySelectorAll("p.pageTitle")
                                .forEach((el) => {
                                    el.textContent = "Your Reminders";
                                });
                        });
                    });

                if (data.timezone !== null) {
                    botTimezone = data.timezone;
                }

                update_times();
            }
        });

    fetch("/dashboard/api/user/guilds")
        .then((response) => response.json())
        .then((data) => {
            if (data.error) {
                show_error(data.error);
            } else {
                const $template = document.getElementById("guildListEntry");

                for (let guild of data) {
                    document
                        .querySelectorAll(".guildList")
                        .forEach((element) => {
                            const $clone = $template.content.cloneNode(true);
                            const $anchor = $clone.querySelector("a");

                            $anchor.innerHTML = $clone
                                .querySelector("a")
                                .innerHTML.replace("%guildname%", guild.name);
                            $anchor.dataset["guild"] = guild.id;
                            $anchor.dataset["name"] = guild.name;

                            $anchor.addEventListener("click", async (e) => {
                                e.preventDefault();

                                $loader.classList.remove("is-hidden");

                                switch_pane($anchor.dataset["pane"]);

                                reset_guild_pane();

                                fetch_roles($anchor.dataset["guild"]);

                                await fetch(
                                    `/dashboard/api/guild/${$anchor.dataset["guild"]}/channels`
                                )
                                    .then((response) => response.json())
                                    .then((data) => {
                                        if (data.error) {
                                            if (
                                                data.error ===
                                                "Bot not in guild"
                                            ) {
                                                switch_pane("guild-error");
                                            } else {
                                                show_error(data.error);
                                            }
                                        } else {
                                            channels = data;

                                            document
                                                .querySelectorAll(
                                                    "select.channel-selector"
                                                )
                                                .forEach(set_channels);
                                        }
                                    });

                                fetch_reminders($anchor.dataset["guild"]);

                                document
                                    .querySelectorAll("p.pageTitle")
                                    .forEach((el) => {
                                        el.textContent =
                                            $anchor.dataset["name"] +
                                            " Reminders";
                                    });
                                document
                                    .querySelectorAll("select.channel-selector")
                                    .forEach((el) => {
                                        el.addEventListener("change", (e) => {
                                            update_select(e.target);
                                        });
                                    });
                                $anchor.classList.add("is-active");
                                resize_textareas();

                                $loader.classList.add("is-hidden");
                            });

                            element.append($clone);
                        });
                }
            }
        });

    $loader.classList.add("is-hidden");
});

function set_channels(element) {
    for (let channel of channels) {
        let newOption = document.createElement("option");

        newOption.value = channel.id;
        newOption.textContent = channel.name;
        if (channel.webhook_avatar !== null) {
            newOption.dataset["webhookAvatar"] = channel.webhook_avatar;
        }
        if (channel.webhook_name !== null) {
            newOption.dataset["webhookName"] = channel.webhook_name;
        }

        element.appendChild(newOption);
    }

    update_select(element);
}

let $createReminder = document.querySelector("#reminderCreator");

$createReminder
    .querySelector("button#createReminder")
    .addEventListener("click", () => {
        // create reminder object
        let seconds =
            parseInt(
                $createReminder.querySelector('input[name="interval_seconds"]')
                    .value
            ) || null;
        let months =
            parseInt(
                $createReminder.querySelector('input[name="interval_months"]')
                    .value
            ) || null;

        let rgb_color = window.getComputedStyle(
            $createReminder.querySelector("div.discord-embed")
        ).borderLeftColor;
        let rgb = rgb_color.match(/\d+/g);
        let color = colorToInt(
            parseInt(rgb[0]),
            parseInt(rgb[1]),
            parseInt(rgb[2])
        );

        let utc_time = luxon.DateTime.fromISO(
            $createReminder.querySelector('input[name="time"]').value
        ).setZone("UTC");

        let reminder = {
            avatar: $createReminder.querySelector("img.discord-avatar").src,
            channel: $createReminder.querySelector("select.channel-selector")
                .value,
            content: $createReminder.querySelector("textarea#messageContent")
                .value,
            embed_author_url: $createReminder.querySelector(
                "img.embed_author_url"
            ).src,
            embed_author: $createReminder.querySelector("textarea#embedAuthor")
                .value,
            embed_color: color,
            embed_description: $createReminder.querySelector(
                "textarea#embedDescription"
            ).value,
            embed_footer: $createReminder.querySelector("textarea#embedFooter")
                .value,
            embed_footer_url: $createReminder.querySelector(
                "img.embed_footer_url"
            ).src,
            embed_image_url: $createReminder.querySelector(
                "img.embed_image_url"
            ).src,
            embed_thumbnail_url: $createReminder.querySelector(
                "img.embed_thumbnail_url"
            ).src,
            embed_title: $createReminder.querySelector("textarea#embedTitle")
                .value,
            enabled: true,
            expires: null,
            interval_seconds: seconds,
            interval_months: months,
            name: $createReminder.querySelector('input[name="name"]').value,
            pin: $createReminder.querySelector('input[name="pin"]').checked,
            restartable: false,
            tts: $createReminder.querySelector('input[name="tts"]').checked,
            username: $createReminder.querySelector("input#reminderUsername")
                .value,
            utc_time: utc_time.toFormat("yyyy-LL-dd'T'HH:mm:ss"),
        };

        // send to server
        let guild = document.querySelector(".guildList a.is-active").dataset[
            "guild"
        ];

        fetch(`/dashboard/api/guild/${guild}/reminders`, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify(reminder),
        })
            .then((response) => response.json())
            .then((data) => console.log(data));

        // process response

        // reset inputs
    });

document.querySelectorAll("textarea.autoresize").forEach((element) => {
    element.addEventListener("input", () => {
        element.style.height = "";
        element.style.height = element.scrollHeight + 3 + "px";
    });
});

let $img;
const $urlModal = document.querySelector("div#addImageModal");
const $urlInput = $urlModal.querySelector("input");

$urlModal.querySelector("button.is-success").addEventListener("click", () => {
    $img.src = $urlInput.value;

    $urlInput.value = "";
    $urlModal.classList.remove("is-active");
});

document.querySelectorAll("button.close-modal").forEach((element) => {
    element.addEventListener("click", () => {
        let $modal = element.closest("div.modal");

        $urlInput.value = "";

        $modal.classList.remove("is-active");
    });
});

document.querySelectorAll(".customizable").forEach((element) => {
    element.querySelector("a").addEventListener("click", (e) => {
        e.preventDefault();

        $img = element.querySelector("img");

        $urlModal.classList.toggle("is-active");
    });
});

document.querySelectorAll("a.icon-toggle").forEach((element) => {
    element.addEventListener("click", (e) => {
        e.preventDefault();

        element.classList.toggle("is-active");
    });
});

let $showButton = document.querySelector("button#showReminderCreator");

$showButton.addEventListener("click", () => {
    $showButton
        .querySelector("span.icon i")
        .classList.toggle("fa-chevron-right");
    $showButton
        .querySelector("span.icon i")
        .classList.toggle("fa-chevron-down");
    document.querySelector("div#reminderCreator").classList.toggle("is-hidden");
});

function register_interval_hide() {
    let $showInterval = document.querySelectorAll("a.intervalLabel");

    $showInterval.forEach((element) => {
        element.addEventListener("click", () => {
            element.querySelector("i").classList.toggle("fa-chevron-right");
            element.querySelector("i").classList.toggle("fa-chevron-down");
            element.nextElementSibling.classList.toggle("is-hidden");
        });
    });
}

const fileInput = document.querySelectorAll("input[type=file]");

fileInput.forEach((element) => {
    element.addEventListener("change", () => {
        if (element.files.length > 0) {
            const fileName = element.parentElement.querySelector(".file-label");
            fileName.textContent = element.files[0].name;
        }
    });
});

function check_embed_fields() {
    document.querySelectorAll(".discord-field-title").forEach((element) => {
        const $template = document.querySelector("template#embedFieldTemplate");
        const $complement = element.parentElement.querySelector(
            ".discord-field-value"
        );

        // when the user clicks out of the field title and if the field title/value are empty, remove the field
        element.addEventListener("blur", () => {
            if (
                element.value === "" &&
                $complement.value === "" &&
                element.parentElement.nextElementSibling !== null
            ) {
                element.parentElement.remove();
            }
        });

        $complement.addEventListener("blur", () => {
            if (
                element.value === "" &&
                $complement.value === "" &&
                element.parentElement.nextElementSibling !== null
            ) {
                element.parentElement.remove();
            }
        });

        // when the user inputs into the end field, create a new field after it
        element.addEventListener("input", () => {
            if (
                element.value !== "" &&
                $complement.value !== "" &&
                element.parentElement.nextElementSibling === null
            ) {
                const $clone = $template.content.cloneNode(true);
                element.parentElement.parentElement.append($clone);
            }
        });

        $complement.addEventListener("input", () => {
            if (
                element.value !== "" &&
                $complement.value !== "" &&
                element.parentElement.nextElementSibling === null
            ) {
                const $clone = $template.content.cloneNode(true);
                element.parentElement.parentElement.append($clone);
            }
        });
    });
}

document.addEventListener("DOMNodeInserted", () => {
    document.querySelectorAll("div.mobile-sidebar a").forEach((element) => {
        element.addEventListener("click", (e) => {
            document
                .getElementById("mobileSidebar")
                .classList.remove("is-active");
            document.querySelectorAll(".navbar-burger").forEach((el) => {
                el.classList.remove("is-active");
            });
        });
    });

    check_embed_fields();
    resize_textareas();
});
