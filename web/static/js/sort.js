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
        for (let child in guildReminders.children) {
        }
    } else {
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

document.querySelector("#orderBy").addEventListener("change", (element) => {
    sort_by(element.value);
});

document.addEventListener("remindersLoaded", () => {
    let select = document.querySelector("#orderBy");

    sort_by(select.value);
});
