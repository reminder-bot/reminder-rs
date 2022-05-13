function collapse_all() {
    document.querySelectorAll("div.reminderContent:not(.creator)").forEach((el) => {
        el.classList.add("is-collapsed");
    });
}

function expand_all() {
    document.querySelectorAll("div.reminderContent:not(.creator)").forEach((el) => {
        el.classList.remove("is-collapsed");
    });
}

const expandAll = document.querySelector("#expandAll");

expandAll.addEventListener("change", (ev) => {
    if (ev.target.value === "expand") {
        expand_all();
    } else if (ev.target.value === "collapse") {
        collapse_all();
    }

    ev.target.value = "";
});
