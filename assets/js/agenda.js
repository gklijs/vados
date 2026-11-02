// vados agenda: every event is in the page, and this hides those that have
// ended, so a page cached for days stays right and works without a script.
(function () {
    const agenda = document.querySelector("[data-agenda]")
    if (agenda == null) {
        return
    }
    const now = Date.now()
    const past = Array.from(agenda.querySelectorAll("[data-event-end]")).filter(function (event) {
        return Date.parse(event.dataset.eventEnd) < now
    })
    const total = agenda.querySelectorAll("[data-event-end]").length
    const toggle = agenda.querySelector("[data-agenda-toggle]")
    const none = agenda.querySelector("[data-agenda-none]")
    let showing = false
    const apply = function () {
        past.forEach(function (event) {
            event.hidden = !showing
        })
        if (none != null) {
            none.hidden = showing || past.length < total
        }
        if (toggle != null) {
            toggle.textContent = showing ? toggle.dataset.hide : toggle.dataset.show
            toggle.setAttribute("aria-expanded", String(showing))
        }
    }
    if (toggle != null && past.length > 0) {
        toggle.hidden = false
        toggle.addEventListener("click", function () {
            showing = !showing
            apply()
        })
    }
    apply()
})()
