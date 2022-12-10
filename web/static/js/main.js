let colorPicker = new iro.ColorPicker("#colorpicker");
let $discordFrame;
const $loader = document.querySelector("#loader");
const $colorPickerModal = document.querySelector("div#pickColorModal");
const $colorPickerInput = $colorPickerModal.querySelector("input");
const $deleteReminderBtn = document.querySelector("#delete-reminder-confirm");
const $reminderTemplate = document.querySelector("template#guildReminder");
const $embedFieldTemplate = document.querySelector("template#embedFieldTemplate");
const $createReminder = document.querySelector("#reminderCreator");
const $createReminderBtn = $createReminder.querySelector("button#createReminder");
const $createTemplateBtn = $createReminder.querySelector("button#createTemplate");
const $loadTemplateBtn = document.querySelector("button#load-template");
const $deleteTemplateBtn = document.querySelector("button#delete-template");
const $templateSelect = document.querySelector("select#templateSelect");
const $exportBtn = document.querySelector("button#export-data");
const $importBtn = document.querySelector("button#import-data");
const $downloader = document.querySelector("a#downloader");
const $uploader = document.querySelector("input#uploader");

let channels = [];
let guildNames = {};
let roles = [];
let templates = {};
let mentions = new Tribute({
    values: [],
    allowSpaces: true,
    selectTemplate: (item) => {
        return `<@&${item.original.value}>`;
    },
});

let globalPatreon = false;
let guildPatreon = false;

function guildId() {
    return document.querySelector(".guildList a.is-active").dataset["guild"];
}

function colorToInt(r, g, b) {
    return (r << 16) + (g << 8) + b;
}

function intToColor(i) {
    return `#${i.toString(16).padStart(6, "0")}`;
}

function switch_pane(selector) {
    document.querySelectorAll("aside a").forEach((el) => {
        el.classList.remove("is-active");
    });
    document.querySelectorAll("div.is-main-content > section").forEach((el) => {
        el.classList.add("is-hidden");
    });

    document.getElementById(selector).classList.remove("is-hidden");
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

async function fetch_patreon(guild_id) {
    fetch(`/dashboard/api/guild/${guild_id}/patreon`)
        .then((response) => response.json())
        .then((data) => {
            if (data.error) {
                show_error(data.error);
            } else {
                return data.patreon;
            }
        });
}

function fetch_roles(guild_id) {
    fetch(`/dashboard/api/guild/${guild_id}/roles`)
        .then((response) => response.json())
        .then((data) => {
            if (data.error) {
                show_error(data.error);
            } else {
                let values = Array.from(
                    data.map((role) => {
                        return {
                            key: role.name,
                            value: role.id,
                        };
                    })
                );

                mentions.collection[0].values = values;
            }
        });
}

function fetch_templates(guild_id) {
    fetch(`/dashboard/api/guild/${guild_id}/templates`)
        .then((response) => response.json())
        .then((data) => {
            if (data.error) {
                show_error(data.error);
            } else {
                templates = {};

                const select = document.querySelector("#templateSelect");
                select.innerHTML = "";
                for (let template of data) {
                    templates[template["id"]] = template;

                    let option = document.createElement("option");
                    option.value = template["id"];
                    option.textContent = template["name"];

                    select.appendChild(option);
                }
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

                    mentions.attach(newFrame.querySelector("textarea"));

                    deserialize_reminder(reminder, newFrame, "load");

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

async function serialize_reminder(node, mode) {
    let interval, utc_time, expiration_time;

    if (mode !== "template") {
        interval = get_interval(node);

        utc_time = luxon.DateTime.fromISO(
            node.querySelector('input[name="time"]').value
        ).setZone("UTC");
        if (utc_time.invalid) {
            return { error: "Time provided invalid." };
        } else {
            utc_time = utc_time.toFormat("yyyy-LL-dd'T'HH:mm:ss");
        }

        expiration_time = luxon.DateTime.fromISO(
            node.querySelector('input[name="time"]').value
        ).setZone("UTC");
        if (expiration_time.invalid) {
            return { error: "Expiration provided invalid." };
        } else {
            expiration_time = expiration_time.toFormat("yyyy-LL-dd'T'HH:mm:ss");
        }
    }

    let rgb_color = window.getComputedStyle(
        node.querySelector("div.discord-embed")
    ).borderLeftColor;
    let rgb = rgb_color.match(/\d+/g);
    let color = colorToInt(parseInt(rgb[0]), parseInt(rgb[1]), parseInt(rgb[2]));

    let fields = [
        ...node.querySelectorAll("div.embed-multifield-box div.embed-field-box"),
    ]
        .map((el) => {
            return {
                title: el.querySelector("textarea.discord-field-title").value,
                value: el.querySelector("textarea.discord-field-value").value,
                inline: el.dataset["inlined"] === "1",
            };
        })
        .filter(({ title, value, inline }) => title.length + value.length > 0);

    let attachment = null;
    let attachment_name = null;

    if (node.querySelector('input[name="attachment"]').files.length > 0) {
        let file = node.querySelector('input[name="attachment"]').files[0];

        if (file.size >= 8 * 1024 * 1024) {
            return { error: "File too large." };
        }

        attachment = await new Promise((resolve) => {
            let fileReader = new FileReader();
            fileReader.onload = (e) => resolve(fileReader.result);
            fileReader.readAsDataURL(file);
        });
        attachment = attachment.split(",")[1];
        attachment_name = file.name;
    }

    let uid = "";
    if (mode === "edit") {
        uid = node.closest(".reminderContent").dataset["uid"];
    }

    let enabled = null;
    if (mode === "create") {
        enabled = true;
    }

    const content = node.querySelector('textarea[name="content"]').value;
    const embed_author_url = has_source(node.querySelector("img.embed_author_url").src);
    const embed_author = node.querySelector('textarea[name="embed_author"]').value;
    const embed_description = node.querySelector(
        'textarea[name="embed_description"]'
    ).value;
    const embed_footer = node.querySelector('textarea[name="embed_footer"]').value;
    const embed_footer_url = has_source(node.querySelector("img.embed_footer_url").src);
    const embed_image_url = has_source(node.querySelector("img.embed_image_url").src);
    const embed_thumbnail_url = has_source(
        node.querySelector("img.embed_thumbnail_url").src
    );
    const embed_title = node.querySelector('textarea[name="embed_title"]').value;

    if (
        attachment === null &&
        content.length == 0 &&
        embed_author_url === null &&
        embed_author.length == 0 &&
        embed_description.length == 0 &&
        embed_footer.length == 0 &&
        embed_footer_url === null &&
        embed_image_url === null &&
        embed_thumbnail_url === null
    ) {
        return { error: "Reminder needs content." };
    }

    return {
        // if we're creating a reminder, ignore this field
        uid: uid,
        // if we're editing a reminder, ignore this field
        enabled: enabled,
        restartable: false,
        attachment: attachment,
        attachment_name: attachment_name,
        avatar: has_source(node.querySelector("img.discord-avatar").src),
        channel: node.querySelector("select.channel-selector").value,
        content: content,
        embed_author_url: embed_author_url,
        embed_author: embed_author,
        embed_color: color,
        embed_description: embed_description,
        embed_footer: embed_footer,
        embed_footer_url: embed_footer_url,
        embed_image_url: embed_image_url,
        embed_thumbnail_url: embed_thumbnail_url,
        embed_title: embed_title,
        embed_fields: fields,
        expires: expiration_time,
        interval_seconds: mode !== "template" ? interval.seconds : null,
        interval_days: mode !== "template" ? interval.days : null,
        interval_months: mode !== "template" ? interval.months : null,
        name: node.querySelector('input[name="name"]').value,
        tts: node.querySelector('input[name="tts"]').checked,
        username: node.querySelector('input[name="username"]').value,
        utc_time: utc_time,
    };
}

function deserialize_reminder(reminder, frame, mode) {
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

    if (mode !== "template") {
        if (reminder["interval_seconds"]) update_interval(frame);

        let $enableBtn = frame.querySelector(".disable-enable");
        $enableBtn.dataset["action"] = reminder["enabled"] ? "disable" : "enable";

        let timeInput = frame.querySelector('input[name="time"]');
        let localTime = luxon.DateTime.fromISO(reminder["utc_time"], {
            zone: "UTC",
        }).setZone(timezone);
        timeInput.value = localTime.toFormat("yyyy-LL-dd'T'HH:mm:ss");

        if (reminder["expires"]) {
            let expiresInput = frame.querySelector('input[name="time"]');
            let expiresTime = luxon.DateTime.fromISO(reminder["expires"], {
                zone: "UTC",
            }).setZone(timezone);
            expiresInput.value = expiresTime.toFormat("yyyy-LL-dd'T'HH:mm:ss");
        }
    }
}

document.addEventListener("guildSwitched", async (e) => {
    $loader.classList.remove("is-hidden");

    document
        .querySelectorAll(".patreon-only")
        .forEach((el) => el.classList.add("is-locked"));

    let $anchor = document.querySelector(
        `.switch-pane[data-guild="${e.detail.guild_id}"]`
    );

    switch_pane($anchor.dataset["pane"]);
    reset_guild_pane();
    $anchor.classList.add("is-active");

    if (globalPatreon || (await fetch_patreon(e.detail.guild_id))) {
        document
            .querySelectorAll(".patreon-only")
            .forEach((el) => el.classList.remove("is-locked"));
    }

    fetch_roles(e.detail.guild_id);
    fetch_templates(e.detail.guild_id);
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

    $loader.classList.add("is-hidden");
});

document.addEventListener("channelsLoaded", () => {
    document.querySelectorAll("select.channel-selector").forEach(set_channels);
});

document.addEventListener("remindersLoaded", (event) => {
    const guild = guildId();

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

            let reminder = await serialize_reminder(node, "edit");
            if (reminder.error) {
                show_error(reminder.error);
                return;
            }

            let guild = guildId();

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
    let guild = guildId();

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

document.querySelectorAll(".show-modal").forEach((element) => {
    element.addEventListener("click", (e) => {
        e.preventDefault();
        document.getElementById(element.dataset["modal"]).classList.toggle("is-active");
    });
});

document.addEventListener("DOMContentLoaded", () => {
    $loader.classList.remove("is-hidden");

    mentions.attach(document.querySelectorAll("textarea"));

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

                globalPatreon = data.patreon;

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
                    guildNames[guild.id] = guild.name;

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
                        $anchor.href = `/dashboard/${guild.id}?name=${guild.name}`;

                        $anchor.addEventListener("click", async (e) => {
                            e.preventDefault();
                            window.history.pushState({}, "", `/dashboard/${guild.id}`);
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

                const matches = window.location.href.match(/dashboard\/(\d+)/);
                if (matches) {
                    let id = matches[1];
                    let name = guildNames[id];

                    const event = new CustomEvent("guildSwitched", {
                        detail: {
                            guild_name: name,
                            guild_id: id,
                        },
                    });

                    document.dispatchEvent(event);
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

$uploader.addEventListener("change", (ev) => {
    const urlTail = document.querySelector('input[name="exportSelect"]:checked').value;

    new Promise((resolve) => {
        let fileReader = new FileReader();
        fileReader.onload = (e) => resolve(fileReader.result);
        fileReader.readAsDataURL($uploader.files[0]);
    }).then((dataUrl) => {
        fetch(`/dashboard/api/guild/${guildId()}/export/${urlTail}`, {
            method: "PUT",
            body: JSON.stringify({ body: dataUrl.split(",")[1] }),
        }).then(() => {
            delete $uploader.files[0];
        });
    });
});

$importBtn.addEventListener("click", () => {
    $uploader.click();
});

$exportBtn.addEventListener("click", () => {
    const urlTail = document.querySelector('input[name="exportSelect"]:checked').value;

    fetch(`/dashboard/api/guild/${guildId()}/export/${urlTail}`)
        .then((response) => response.json())
        .then((data) => {
            $downloader.href =
                "data:text/plain;charset=utf-8," + encodeURIComponent(data.body);
            $downloader.click();
        });
});

$createReminderBtn.addEventListener("click", async () => {
    $createReminderBtn.querySelector("span.icon > i").classList = [
        "fas fa-spinner fa-spin",
    ];

    let reminder = await serialize_reminder($createReminder, "create");
    if (reminder.error) {
        show_error(reminder.error);
        return;
    }

    let guild = guildId();

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

                $createReminderBtn.querySelector("span.icon > i").classList = [
                    "fas fa-sparkles",
                ];
            } else {
                const $reminderBox = document.querySelector("div#guildReminders");
                let newFrame = $reminderTemplate.content.cloneNode(true);

                newFrame.querySelector(".reminderContent").dataset["uid"] = data["uid"];

                deserialize_reminder(data, newFrame, "load");

                $reminderBox.appendChild(newFrame);

                data.node = $reminderBox.lastElementChild;

                document.dispatchEvent(
                    new CustomEvent("remindersLoaded", {
                        detail: [data],
                    })
                );

                $createReminderBtn.querySelector("span.icon > i").classList = [
                    "fas fa-check",
                ];

                window.setTimeout(() => {
                    $createReminderBtn.querySelector("span.icon > i").classList = [
                        "fas fa-sparkles",
                    ];
                }, 1500);
            }
        });
});

$createTemplateBtn.addEventListener("click", async () => {
    $createTemplateBtn.querySelector("span.icon > i").classList = [
        "fas fa-spinner fa-spin",
    ];

    let reminder = await serialize_reminder($createReminder, "template");
    let guild = guildId();

    fetch(`/dashboard/api/guild/${guild}/templates`, {
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
                $createTemplateBtn.querySelector("span.icon > i").classList = [
                    "fas fa-file-spreadsheet",
                ];
            } else {
                fetch_templates(guildId());

                $createTemplateBtn.querySelector("span.icon > i").classList = [
                    "fas fa-check",
                ];

                window.setTimeout(() => {
                    $createTemplateBtn.querySelector("span.icon > i").classList = [
                        "fas fa-file-spreadsheet",
                    ];
                }, 1500);
            }
        });
});

$loadTemplateBtn.addEventListener("click", (ev) => {
    deserialize_reminder(
        templates[parseInt($templateSelect.value)],
        $createReminder,
        "template"
    );
});

$deleteTemplateBtn.addEventListener("click", (ev) => {
    fetch(`/dashboard/api/guild/${guildId()}/templates`, {
        method: "DELETE",
        headers: {
            "Content-Type": "application/json",
        },
        body: JSON.stringify({ id: parseInt($templateSelect.value) }),
    })
        .then((response) => response.json())
        .then((data) => {
            if (data.error) {
                show_error(data.error);
            } else {
                $templateSelect
                    .querySelector(`option[value="${$templateSelect.value}"]`)
                    .remove();
            }
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

    const fileInput = document.querySelectorAll("input.file-input[type=file]");

    fileInput.forEach((element) => {
        element.addEventListener("change", () => {
            if (element.files.length > 0) {
                const fileName = element.parentElement.querySelector(".file-label");
                fileName.textContent = element.files[0].name;
            }
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
});

document.addEventListener("click", (ev) => {
    if (ev.target.closest("button.inline-btn") !== null) {
        let inlined = ev.target.closest(".embed-field-box").dataset["inlined"];
        ev.target.closest(".embed-field-box").dataset["inlined"] =
            inlined == "1" ? "0" : "1";
    }
});
