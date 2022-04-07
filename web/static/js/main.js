let colorPicker = new iro.ColorPicker("#colorpicker");
let $discordFrame;
const $loader = document.querySelector("#loader");
const $colorPickerModal = document.querySelector("div#pickColorModal");
const $colorPickerInput = $colorPickerModal.querySelector("input");
const $deleteReminderBtn = document.querySelector("#delete-reminder-confirm");
const $reminderTemplate = document.querySelector("template#guildReminder");
const $embedFieldTemplate = document.querySelector("template#embedFieldTemplate");

let channels;
let roles;

function colorToInt(r, g, b) {
    return (r << 16) + (g << 8) + b;
}

function intToColor(i) {
    return `#${i.toString(16).padStart(6, "0")}`;
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
        sel.closest("div.reminderContent").querySelector("img.discord-avatar").src =
            sel.selectedOptions[0].dataset["webhookAvatar"];
    } else {
        sel.closest("div.reminderContent").querySelector("img.discord-avatar").src = "";
    }
    if (sel.selectedOptions[0].dataset["webhookName"]) {
        sel.closest("div.reminderContent").querySelector("input.discord-username").value =
            sel.selectedOptions[0].dataset["webhookName"];
    } else {
        sel.closest("div.reminderContent").querySelector("input.discord-username").value =
            "";
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

async function fetch_channels(guild_id) {
    const event = new Event("channelsLoading");
    document.dispatchEvent(event);

    await fetch(`/dashboard/api/guild/${guild_id}/channels`)
        .then((response) => response.json())
        .then((data) => {
            if (data.error) {
                if (data.error === "Bot not in guild") {
                    switch_pane("guild-error");
                } else {
                    show_error(data.error);
                }
            } else {
                channels = data;
            }
        })
        .then(() => {
            const event = new Event("channelsLoaded");
            document.dispatchEvent(event);
        });
}

async function fetch_reminders(guild_id) {
    document.dispatchEvent(new Event("remindersLoading"));

    const $reminderBox = document.querySelector("div#guildReminders");

    // reset div contents
    $reminderBox.innerHTML = "";

    // fetch reminders
    await fetch(`/dashboard/api/guild/${guild_id}/reminders`)
        .then((response) => response.json())
        .then((data) => {
            if (data.error) {
                show_error(data.error);
            } else {
                for (let reminder of data) {
                    let newFrame = $reminderTemplate.content.cloneNode(true);

                    newFrame.querySelector(".reminderContent").dataset["uid"] =
                        reminder["uid"];

                    deserialize_reminder(reminder, newFrame);

                    $reminderBox.appendChild(newFrame);

                    reminder.node = $reminderBox.lastElementChild;
                }

                const remindersLoadedEvent = new CustomEvent("remindersLoaded", {
                    detail: data,
                });

                document.dispatchEvent(remindersLoadedEvent);
            }
        });
}

async function serialize_reminder(node) {
    let interval = get_interval(node);

    let rgb_color = window.getComputedStyle(
        node.querySelector("div.discord-embed")
    ).borderLeftColor;
    let rgb = rgb_color.match(/\d+/g);
    let color = colorToInt(parseInt(rgb[0]), parseInt(rgb[1]), parseInt(rgb[2]));

    let utc_time = luxon.DateTime.fromISO(
        node.querySelector('input[name="time"]').value
    ).setZone("UTC");

    if (utc_time.invalid) {
        return { error: "Time provided invalid." };
    }

    let fields = [
        ...node.querySelectorAll("div.embed-multifield-box div.embed-field-box"),
    ]
        .map((el) => {
            return {
                title: el.querySelector("textarea#embedFieldTitle").value,
                value: el.querySelector("textarea#embedFieldValue").value,
                inline: el.dataset["inlined"] === "1",
            };
        })
        .filter(({ title, value, inline }) => title.length + value.length > 0);

    let attachment = null;
    let attachment_name = null;

    if (node.querySelector('input[name="attachment"]').files.length > 0) {
        let file = node.querySelector('input[name="attachment"]').files[0];

        attachment = await new Promise((resolve) => {
            let fileReader = new FileReader();
            fileReader.onload = (e) => resolve(fileReader.result);
            fileReader.readAsDataURL(file);
        });
        attachment = attachment.split(",")[1];
        attachment_name = file.name;
    }

    const reminderContent = node.closest(".reminderContent");

    return {
        // if we're creating a reminder, ignore this field
        uid: reminderContent !== null ? reminderContent.dataset["uid"] : "",
        // if we're editing a reminder, ignore this field
        enabled: reminderContent !== null ? null : true,
        restartable: false,
        attachment: attachment,
        attachment_name: attachment_name,
        avatar: has_source(node.querySelector("img.discord-avatar").src),
        channel: node.querySelector("select.channel-selector").value,
        content: node.querySelector('textarea[name="content"]').value,
        embed_author_url: has_source(node.querySelector("img.embed_author_url").src),
        embed_author: node.querySelector('textarea[name="embed_author"]').value,
        embed_color: color,
        embed_description: node.querySelector('textarea[name="embed_description"]').value,
        embed_footer: node.querySelector('textarea[name="embed_footer"]').value,
        embed_footer_url: has_source(node.querySelector("img.embed_footer_url").src),
        embed_image_url: has_source(node.querySelector("img.embed_image_url").src),
        embed_thumbnail_url: has_source(
            node.querySelector("img.embed_thumbnail_url").src
        ),
        embed_title: node.querySelector('textarea[name="embed_title"]').value,
        embed_fields: fields,
        expires: null,
        interval_seconds: interval.seconds,
        interval_months: interval.months,
        name: node.querySelector('input[name="name"]').value,
        pin: node.querySelector('input[name="pin"]').checked,
        tts: node.querySelector('input[name="tts"]').checked,
        username: node.querySelector('input[name="username"]').value,
        utc_time: utc_time.toFormat("yyyy-LL-dd'T'HH:mm:ss"),
    };
}

function deserialize_reminder(reminder, frame) {
    // populate channels
    set_channels(frame.querySelector("select.channel-selector"));

    // populate majority of items
    for (let prop in reminder) {
        if (reminder.hasOwnProperty(prop) && reminder[prop] !== null) {
            if (prop === "attachment") {
            } else if (prop === "attachment_name") {
                frame.querySelector(".file-cta > .file-label").textContent =
                    reminder[prop];
            } else {
                let $input = frame.querySelector(`*[name="${prop}"]`);
                let $image = frame.querySelector(`img.${prop}`);

                if ($input !== null) {
                    $input.value = reminder[prop];
                } else if ($image !== null) {
                    $image.src = reminder[prop];
                }
            }
        }
    }

    const lastChild = frame.querySelector("div.embed-multifield-box .embed-field-box");

    for (let field of reminder["embed_fields"]) {
        let embed_field = $embedFieldTemplate.content.cloneNode(true);
        embed_field.querySelector("textarea.discord-field-title").value = field["title"];
        embed_field.querySelector("textarea.discord-field-value").value = field["value"];
        embed_field.querySelector(".embed-field-box").dataset["inlined"] = field["inline"]
            ? "1"
            : "0";

        frame
            .querySelector("div.embed-multifield-box")
            .insertBefore(embed_field, lastChild);
    }

    if (reminder["interval_seconds"] !== null) update_interval(frame);

    let $enableBtn = frame.querySelector(".disable-enable");
    $enableBtn.dataset["action"] = reminder["enabled"] ? "disable" : "enable";

    let timeInput = frame.querySelector('input[name="time"]');
    let localTime = luxon.DateTime.fromISO(reminder["utc_time"], { zone: "UTC" }).setZone(
        timezone
    );
    timeInput.value = localTime.toFormat("yyyy-LL-dd'T'HH:mm:ss");
}

document.addEventListener("guildSwitched", async (e) => {
    $loader.classList.remove("is-hidden");

    let $anchor = document.querySelector(
        `.switch-pane[data-guild="${e.detail.guild_id}"]`
    );

    switch_pane($anchor.dataset["pane"]);
    $anchor.classList.add("is-active");

    reset_guild_pane();

    fetch_roles(e.detail.guild_id);
    await fetch_channels(e.detail.guild_id);
    fetch_reminders(e.detail.guild_id);

    document.querySelectorAll("p.pageTitle").forEach((el) => {
        el.textContent = `${e.detail.guild_name} Reminders`;
    });
    document.querySelectorAll("select.channel-selector").forEach((el) => {
        el.addEventListener("change", (e) => {
            update_select(e.target);
        });
    });

    resize_textareas();

    $loader.classList.add("is-hidden");
});

document.addEventListener("channelsLoaded", () => {
    document.querySelectorAll("select.channel-selector").forEach(set_channels);
});

document.addEventListener("remindersLoaded", (event) => {
    const guild = document.querySelector(".guildList a.is-active").dataset["guild"];

    for (let reminder of event.detail) {
        let node = reminder.node;

        node.querySelector("button.hide-box").addEventListener("click", () => {
            node.closest(".reminderContent").classList.toggle("is-collapsed");
        });

        node.querySelector("div.discord-embed").style.borderLeftColor = intToColor(
            reminder.embed_color
        );

        const enableBtn = node.querySelector(".disable-enable");
        enableBtn.addEventListener("click", () => {
            let enable = enableBtn.dataset["action"] === "enable";

            fetch(`/dashboard/api/guild/${guild}/reminders`, {
                method: "PATCH",
                body: JSON.stringify({
                    uid: reminder["uid"],
                    enabled: enable,
                }),
            })
                .then((response) => response.json())
                .then((data) => {
                    if (data.error) {
                        show_error(data.error);
                    } else {
                        enableBtn.dataset["action"] = data["enabled"]
                            ? "enable"
                            : "disable";
                    }
                });
        });

        node.querySelector("button.delete-reminder").addEventListener("click", () => {
            $deleteReminderBtn.dataset["uid"] = reminder["uid"];
            $deleteReminderBtn.closest(".modal").classList.toggle("is-active");
        });

        const $saveBtn = node.querySelector("button.save-btn");

        $saveBtn.addEventListener("click", async (event) => {
            $saveBtn.querySelector("span.icon > i").classList = [
                "fas fa-spinner fa-spin",
            ];

            let reminder = await serialize_reminder(node);
            if (reminder.error) {
                show_error(reminder.error);
                return;
            }

            let guild = document.querySelector(".guildList a.is-active").dataset["guild"];

            fetch(`/dashboard/api/guild/${guild}/reminders`, {
                method: "PATCH",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(reminder),
            })
                .then((response) => response.json())
                .then((data) => {
                    for (let error of data.errors) show_error(error);
                });

            $saveBtn.querySelector("span.icon > i").classList = ["fas fa-check"];

            window.setTimeout(() => {
                $saveBtn.querySelector("span.icon > i").classList = ["fas fa-save"];
            }, 1500);
        });
    }
});

$deleteReminderBtn.addEventListener("click", () => {
    let guild = document.querySelector(".guildList a.is-active").dataset["guild"];

    fetch(`/dashboard/api/guild/${guild}/reminders`, {
        method: "DELETE",
        body: JSON.stringify({
            uid: $deleteReminderBtn.dataset["uid"],
        }),
    }).then(() => {
        document.querySelector("#deleteReminderModal").classList.remove("is-active");
        fetch_reminders(guild);
    });
});

function show_error(error) {
    document.getElementById("errors").querySelector("span.error-message").textContent =
        error;
    document.getElementById("errors").classList.add("is-active");

    window.setTimeout(() => {
        document.getElementById("errors").classList.remove("is-active");
    }, 5000);
}

$colorPickerInput.value = colorPicker.color.hexString;

$colorPickerInput.addEventListener("input", () => {
    if (/^#[0-9a-fA-F]{6}$/.test($colorPickerInput.value) === true) {
        colorPicker.color.hexString = $colorPickerInput.value;
    }
});

colorPicker.on("color:change", function (color) {
    $colorPickerInput.value = color.hexString;
});

$colorPickerModal.querySelector("button.is-success").addEventListener("click", () => {
    $discordFrame.style.borderLeftColor = colorPicker.color.rgbString;

    $colorPickerModal.classList.remove("is-active");
});

document.querySelectorAll("a.show-modal").forEach((element) => {
    element.addEventListener("click", (e) => {
        e.preventDefault();
        document.getElementById(element.dataset["modal"]).classList.toggle("is-active");
    });
});

document.addEventListener("DOMContentLoaded", () => {
    $loader.classList.remove("is-hidden");

    document.querySelectorAll(".navbar-burger").forEach((el) => {
        el.addEventListener("click", () => {
            const target = el.dataset["target"];
            const $target = document.getElementById(target);

            el.classList.toggle("is-active");
            $target.classList.toggle("is-active");
        });
    });

    let hideBox = document.querySelector("#reminderCreator button.hide-box");
    hideBox.addEventListener("click", () => {
        hideBox.closest(".reminderContent").classList.toggle("is-collapsed");
    });

    fetch("/dashboard/api/user")
        .then((response) => response.json())
        .then((data) => {
            if (data.error) {
                show_error(data.error);
            } else {
                if (data.timezone !== null) botTimezone = data.timezone;

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
                    document.querySelectorAll(".guildList").forEach((element) => {
                        const $clone = $template.content.cloneNode(true);
                        const $anchor = $clone.querySelector("a");

                        let $span = $clone.querySelector("a > span.guild-name");

                        $span.textContent = $span.textContent.replace(
                            "%guildname%",
                            guild.name
                        );
                        $anchor.dataset["guild"] = guild.id;
                        $anchor.dataset["name"] = guild.name;

                        $anchor.addEventListener("click", async (e) => {
                            e.preventDefault();

                            const event = new CustomEvent("guildSwitched", {
                                detail: {
                                    guild_name: guild.name,
                                    guild_id: guild.id,
                                },
                            });

                            document.dispatchEvent(event);
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

function has_source(string) {
    if (string.startsWith(`https://${window.location.hostname}`)) {
        return null;
    } else {
        return string;
    }
}

let $createReminder = document.querySelector("#reminderCreator");
let $createBtn = $createReminder.querySelector("button#createReminder");

$createBtn.addEventListener("click", async () => {
    $createBtn.querySelector("span.icon > i").classList = ["fas fa-spinner fa-spin"];

    let attachment = null;
    let attachment_name = null;

    if ($createReminder.querySelector('input[name="attachment"]').files.length > 0) {
        let file = $createReminder.querySelector('input[name="attachment"]').files[0];

        attachment = await new Promise((resolve) => {
            let fileReader = new FileReader();
            fileReader.onload = (e) => resolve(fileReader.result);
            fileReader.readAsDataURL(file);
        });
        attachment = attachment.split(",")[1];
        attachment_name = file.name;
    }

    let reminder = await serialize_reminder($createReminder);
    if (reminder.error) {
        show_error(reminder.error);
        return;
    }

    let guild = document.querySelector(".guildList a.is-active").dataset["guild"];

    fetch(`/dashboard/api/guild/${guild}/reminders`, {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
        },
        body: JSON.stringify(reminder),
    })
        .then((response) => response.json())
        .then((data) => {
            if (data.error) {
                show_error(data.error);
            } else {
                const $reminderBox = document.querySelector("div#guildReminders");
                let newFrame = $reminderTemplate.content.cloneNode(true);

                newFrame.querySelector(".reminderContent").dataset["uid"] = data["uid"];

                deserialize_reminder(data, newFrame);

                $reminderBox.appendChild(newFrame);

                data.node = $reminderBox.lastElementChild;

                document.dispatchEvent(
                    new CustomEvent("remindersLoaded", {
                        detail: [data],
                    })
                );
            }

            $createBtn.querySelector("span.icon > i").classList = ["fas fa-check"];

            window.setTimeout(() => {
                $createBtn.querySelector("span.icon > i").classList = ["fas fa-sparkles"];
            }, 1500);
        });
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

$urlModal.querySelector("button#setImgUrl").addEventListener("click", () => {
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

document.addEventListener("remindersLoaded", () => {
    document.querySelectorAll(".customizable").forEach((element) => {
        element.querySelector("a").addEventListener("click", (e) => {
            e.preventDefault();

            $img = element.querySelector("img");

            $urlModal.classList.toggle("is-active");
        });
    });

    const fileInput = document.querySelectorAll("input[type=file]");

    fileInput.forEach((element) => {
        element.addEventListener("change", () => {
            if (element.files.length > 0) {
                const fileName = element.parentElement.querySelector(".file-label");
                fileName.textContent = element.files[0].name;
            }
        });
    });

    const $showInterval = document.querySelectorAll("a.intervalLabel");

    $showInterval.forEach((element) => {
        element.addEventListener("click", () => {
            element.querySelector("i").classList.toggle("fa-chevron-right");
            element.querySelector("i").classList.toggle("fa-chevron-down");
            element.nextElementSibling.classList.toggle("is-hidden");
        });
    });

    document.querySelectorAll(".change-color").forEach((element) => {
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
});

function check_embed_fields() {
    document.querySelectorAll(".embed-field-box").forEach((element) => {
        const $titleInput = element.querySelector(".discord-field-title");
        const $valueInput = element.querySelector(".discord-field-value");

        // when the user clicks out of the field title and if the field title/value are empty, remove the field
        $titleInput.addEventListener("blur", () => {
            if (
                $titleInput.value === "" &&
                $valueInput.value === "" &&
                element.nextElementSibling !== null
            ) {
                element.remove();
            }
        });

        $valueInput.addEventListener("blur", () => {
            if (
                $titleInput.value === "" &&
                $valueInput.value === "" &&
                element.nextElementSibling !== null
            ) {
                element.remove();
            }
        });

        // when the user inputs into the end field, create a new field after it
        $titleInput.addEventListener("input", () => {
            if (
                $titleInput.value !== "" &&
                $valueInput.value !== "" &&
                element.nextElementSibling === null
            ) {
                const $clone = $embedFieldTemplate.content.cloneNode(true);
                element.parentElement.append($clone);
            }
        });

        $valueInput.addEventListener("input", () => {
            if (
                $titleInput.value !== "" &&
                $valueInput.value !== "" &&
                element.nextElementSibling === null
            ) {
                const $clone = $embedFieldTemplate.content.cloneNode(true);
                element.parentElement.append($clone);
            }
        });
    });
}

document.addEventListener("DOMNodeInserted", () => {
    document.querySelectorAll("div.mobile-sidebar a").forEach((element) => {
        element.addEventListener("click", (e) => {
            document.getElementById("mobileSidebar").classList.remove("is-active");
            document.querySelectorAll(".navbar-burger").forEach((el) => {
                el.classList.remove("is-active");
            });
        });
    });

    document.querySelectorAll('input[type="datetime-local"]').forEach((el) => {
        let now = luxon.DateTime.now().setZone(timezone);
        el.min = now.toFormat("yyyy-LL-dd'T'HH:mm:ss");
    });

    check_embed_fields();
    resize_textareas();
});

document.addEventListener("click", (ev) => {
    if (ev.target.closest("button.inline-btn") !== null) {
        let inlined = ev.target.closest(".embed-field-box").dataset["inlined"];
        ev.target.closest(".embed-field-box").dataset["inlined"] =
            inlined == "1" ? "0" : "1";
    }
});
