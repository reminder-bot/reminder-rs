let guildReminders = document.querySelector("#guildReminders");

function sort_by(cond) {
    if (cond === "channel") {
        [...guildReminders.children]
            .sort((a, b) => {
                let channel1 = a.querySelector("select.channel-selector").value;
                let channel2 = b.querySelector("select.channel-selector").value;

                return channel1 > channel2 ? 1 : -1;
            })
            .forEach((node) => guildReminders.appendChild(node));

        // go through and add channel categories
        let currentChannelGroup = null;
        for (let child of guildReminders.querySelectorAll("div.reminderContent")) {
            let thisChannelGroup = child.querySelector("select.channel-selector").value;

            if (currentChannelGroup !== thisChannelGroup) {
                let newNode = document.createElement("div");
                newNode.textContent =
                    "#" + channels.find((a) => a.id === thisChannelGroup).name;
                newNode.classList.add("channel-tag");

                guildReminders.insertBefore(newNode, child);

                currentChannelGroup = thisChannelGroup;
            }
        }
    } else {
        // remove any channel tags if previous ordering was by channel
        guildReminders.querySelectorAll("div.channel-tag").forEach((el) => {
            el.remove();
        });

        if (cond === "time") {
            [...guildReminders.children]
                .sort((a, b) => {
                    let time1 = luxon.DateTime.fromISO(
                        a.querySelector('input[name="time"]').value
                    );
                    let time2 = luxon.DateTime.fromISO(
                        b.querySelector('input[name="time"]').value
                    );

                    return time1 > time2 ? 1 : -1;
                })
                .forEach((node) => guildReminders.appendChild(node));
        } else {
            [...guildReminders.children]
                .sort((a, b) => {
                    let name1 = a.querySelector('input[name="name"]').value;
                    let name2 = b.querySelector('input[name="name"]').value;

                    return name1 > name2 ? 1 : -1;
                })
                .forEach((node) => guildReminders.appendChild(node));
        }
    }
}

const selector = document.querySelector("#orderBy");

selector.addEventListener("change", () => {
    sort_by(selector.value);
});

document.addEventListener("remindersLoaded", () => {
    sort_by(selector.value);
});
