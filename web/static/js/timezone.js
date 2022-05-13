let timezone = luxon.DateTime.now().zone.name;
const browserTimezone = luxon.DateTime.now().zone.name;
let botTimezone = "UTC";

function update_times() {
    document.querySelectorAll("span.set-timezone").forEach((element) => {
        element.textContent = timezone;
    });
    document.querySelectorAll("span.set-time").forEach((element) => {
        element.textContent = luxon.DateTime.now().setZone(timezone).toFormat("HH:mm");
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
        element.textContent = luxon.DateTime.now().setZone(botTimezone).toFormat("HH:mm");
    });
}

window.setInterval(() => {
    update_times();
}, 30000);

document.getElementById("set-bot-timezone").addEventListener("click", () => {
    timezone = botTimezone;
    update_times();
});
document.getElementById("set-browser-timezone").addEventListener("click", () => {
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
