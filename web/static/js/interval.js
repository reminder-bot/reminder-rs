function update_input(element) {
    let months = element.querySelector('input[name="interval_months"]');
    let days = element.querySelector('input[name="interval_days"]');
    let hours = element.querySelector('input[name="interval_hours"]');
    let minutes = element.querySelector('input[name="interval_minutes"]');
    let seconds = element.querySelector('input[name="interval_seconds"]');

    months.value = months.value.padStart(1, "0");
    days.value = days.value.padStart(1, "0");
    hours.value = hours.value.padStart(2, "0");
    minutes.value = minutes.value.padStart(2, "0");
    seconds.value = seconds.value.padStart(2, "0");

    if (seconds.value >= 60) {
        let quotient = Math.floor(seconds.value / 60);
        let remainder = seconds.value % 60;

        seconds.value = String(remainder).padStart(2, "0");
        minutes.value = String(Number(minutes.value) + Number(quotient)).padStart(2, "0");
    }
    if (minutes.value >= 60) {
        let quotient = Math.floor(minutes.value / 60);
        let remainder = minutes.value % 60;

        minutes.value = String(remainder).padStart(2, "0");
        hours.value = String(Number(hours.value) + Number(quotient)).padStart(2, "0");
    }
    if (hours.value >= 24) {
        let quotient = Math.floor(hours.value / 24);
        let remainder = hours.value % 24;

        hours.value = String(remainder).padStart(2, "0");
        days.value = Number(days.value) + Number(quotient);
    }
}

let $intervalGroup = document.querySelector(".interval-group");

document.querySelector(".interval-group").addEventListener(
    "blur",
    (ev) => {
        if (ev.target.nodeName !== "BUTTON") update_input($intervalGroup);
    },
    true
);

$intervalGroup.querySelector("button.clear").addEventListener("click", () => {
    $intervalGroup.querySelectorAll("input").forEach((el) => {
        el.value = "";
    });
});

document.addEventListener("remindersLoaded", (event) => {
    for (reminder of event.detail) {
        let $intervalGroup = reminder.node.querySelector(".interval-group");

        $intervalGroup.addEventListener("blur", (ev) => {
            if (ev.target.nodeName !== "BUTTON") update_input($intervalGroup);
        });

        $intervalGroup.querySelector("button.clear").addEventListener("click", () => {
            $intervalGroup.querySelectorAll("input").forEach((el) => {
                el.value = "";
            });
        });
    }
});
