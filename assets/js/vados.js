let burger_menu_active = false
let side_menu_active = false
let burger_menu = document.getElementById("burger-menu")
let main_menu = document.getElementById("main-menu")
let burger_side_menu = document.getElementById("burger-side-menu")
let side_menu = document.getElementById("side-menu-mobile")
const toggleBurgerMenu = function () {
    if (side_menu_active) {
        toggleSideMenu()
    }
    if (burger_menu_active) {
        burger_menu.classList.remove("is-active")
        main_menu.classList.remove("is-active")
        burger_menu.setAttribute("aria-expanded", "false")
    } else {
        burger_menu.classList.add("is-active")
        main_menu.classList.add("is-active")
        burger_menu.setAttribute("aria-expanded", "true")
    }
    burger_menu_active = !burger_menu_active
}
const toggleSideMenu = function () {
    if (burger_menu_active) {
        toggleBurgerMenu()
    }
    if (side_menu_active) {
        burger_side_menu.classList.remove("is-active")
        side_menu.classList.add("is-hidden")
        burger_side_menu.setAttribute("aria-expanded", "false")
    } else {
        burger_side_menu.classList.add("is-active")
        side_menu.classList.remove("is-hidden")
        burger_side_menu.setAttribute("aria-expanded", "true")
    }
    side_menu_active = !side_menu_active
}
if (burger_menu != null) {
    burger_menu.onclick = function () {
        toggleBurgerMenu()
    }
}
if (typeof (burger_side_menu) != 'undefined' && burger_side_menu != null) {
    burger_side_menu.onclick = function () {
        toggleSideMenu()
    }
}

// Main menu dropdowns: the WAI disclosure navigation pattern. Without this
// script the buttons stay hidden and Bulma opens the lists on hover and focus.
const disclosures = Array.from(document.querySelectorAll("[data-nav-disclosure]")).map(function (item) {
    const button = item.querySelector(".nav-disclosure")
    const list = item.querySelector(".navbar-dropdown")
    return {item: item, button: button, list: list}
}).filter(function (d) {
    return d.button != null && d.list != null
})
const setOpen = function (d, open) {
    d.button.setAttribute("aria-expanded", String(open))
    d.item.classList.toggle("is-active", open)
    d.list.hidden = !open
    if (open) {
        keepOnScreen(d.list)
    }
}
// Fits an open list on the screen, whatever the bar's wrapping and the text
// size did to where it starts. Only for a list floating below the bar, not one
// inside an open burger menu. Its CSS max-width is already narrower than the
// screen, so moving it sideways is always enough.
const keepOnScreen = function (list) {
    list.style.left = ""
    list.style.maxHeight = ""
    if (getComputedStyle(list).position !== "absolute") {
        return
    }
    list.classList.remove("is-right")
    const box = list.getBoundingClientRect()
    const width = document.documentElement.clientWidth
    if (box.right > width) {
        list.style.left = (width - box.right) + "px"
    } else if (box.left < 0) {
        list.style.left = (-box.left) + "px"
    }
    list.style.maxHeight = Math.max(window.innerHeight - box.top - 8, 100) + "px"
}
const closeAll = function () {
    disclosures.forEach(function (d) {
        setOpen(d, false)
    })
}
disclosures.forEach(function (d) {
    d.item.classList.remove("is-hoverable")
    d.item.classList.add("is-enhanced")
    d.button.hidden = false
    setOpen(d, false)
    d.button.addEventListener("click", function () {
        const open = d.button.getAttribute("aria-expanded") !== "true"
        closeAll()
        setOpen(d, open)
    })
    d.item.addEventListener("focusout", function (event) {
        // Moving focus to another window keeps the list open: only a move
        // to somewhere else on the page closes it.
        if (event.relatedTarget != null && !d.item.contains(event.relatedTarget)) {
            setOpen(d, false)
        }
    })
})
document.addEventListener("click", function (event) {
    disclosures.forEach(function (d) {
        if (!d.item.contains(event.target)) {
            setOpen(d, false)
        }
    })
})
document.addEventListener("keydown", function (event) {
    if (event.key !== "Escape") {
        return
    }
    const open = disclosures.find(function (d) {
        return d.button.getAttribute("aria-expanded") === "true"
    })
    if (open != null) {
        setOpen(open, false)
        open.button.focus()
    } else if (burger_menu_active) {
        toggleBurgerMenu()
        burger_menu.focus()
    } else if (side_menu_active) {
        toggleSideMenu()
        burger_side_menu.focus()
    }
})

// A menu that wraps makes the fixed bar taller than the room Bulma keeps for
// it, so keep the page clear of whatever height the bar really has.
const nav_bar = document.getElementById("nav-bar")
if (nav_bar != null && burger_menu != null && "ResizeObserver" in window) {
    const followNavBar = function () {
        const root = document.documentElement
        // The open burger menu is part of the bar and floats over the page.
        if (getComputedStyle(burger_menu).display !== "none") {
            root.style.paddingTop = ""
            root.style.scrollPaddingTop = ""
        } else {
            root.style.paddingTop = nav_bar.offsetHeight + "px"
            root.style.scrollPaddingTop = nav_bar.offsetHeight + "px"
        }
    }
    new ResizeObserver(followNavBar).observe(nav_bar)
    followNavBar()
}
