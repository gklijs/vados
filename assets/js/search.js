// vados site search. Nothing is requested until a non-empty search is
// submitted, so visitors who don't search don't download the index.
(function () {
    const form = document.getElementById("search-form")
    const status = document.getElementById("search-status")
    const results = document.getElementById("search-results")
    if (form == null || status == null || results == null) {
        return
    }
    const input = form.querySelector("input")
    const text = form.dataset
    let index = null

    const normalize = function (value) {
        return value.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase()
            .replace(/[^\p{L}\p{N}]+/gu, " ").trim()
    }
    const loadIndex = function () {
        if (index == null) {
            index = fetch(text.index).then(function (response) {
                if (!response.ok) {
                    throw new Error(String(response.status))
                }
                return response.json()
            }).then(function (records) {
                return records.map(function (record) {
                    record.normalTitle = normalize(record.title)
                    record.normalText = normalize(record.title + " " + (record.text || ""))
                    return record
                })
            }).catch(function (error) {
                // Forget the failure, so searching again tries again.
                index = null
                throw error
            })
        }
        return index
    }
    // Every word must match; title matches come first, then the newest.
    const find = function (records, query) {
        const words = normalize(query).split(" ").filter(Boolean)
        return records.filter(function (record) {
            return words.every(function (word) {
                return record.normalText.includes(word)
            })
        }).map(function (record) {
            const inTitle = words.every(function (word) {
                return record.normalTitle.includes(word)
            })
            return {record: record, inTitle: inTitle}
        }).sort(function (a, b) {
            if (a.inTitle !== b.inTitle) {
                return a.inTitle ? -1 : 1
            }
            return (b.record.date || "").localeCompare(a.record.date || "")
        }).map(function (match) {
            return match.record
        })
    }
    const show = function (found) {
        results.replaceChildren.apply(results, found.map(function (record) {
            const item = document.createElement("li")
            item.className = "block"
            const link = document.createElement("a")
            link.href = record.url
            link.textContent = record.title
            item.appendChild(link)
            const about = [record.section, record.dateText].filter(Boolean).join(" · ")
            if (about) {
                const line = document.createElement("p")
                line.className = "is-size-7"
                line.textContent = about
                item.appendChild(line)
            }
            if (record.text) {
                const line = document.createElement("p")
                line.textContent = record.text
                item.appendChild(line)
            }
            return item
        }))
    }
    const search = function (query) {
        results.setAttribute("aria-busy", "true")
        status.textContent = text.searching
        return loadIndex().then(function (records) {
            const found = find(records, query)
            show(found)
            status.textContent = found.length === 0
                ? text.none
                : found.length === 1
                    ? text.one
                    : text.many.replace("{n}", String(found.length))
        }).catch(function () {
            show([])
            status.textContent = text.failed
        }).finally(function () {
            results.removeAttribute("aria-busy")
        })
    }
    form.hidden = false
    const noScript = document.getElementById("search-no-script")
    if (noScript != null) {
        noScript.hidden = true
    }
    form.addEventListener("submit", function (event) {
        event.preventDefault()
        const query = input.value
        const url = new URL(window.location.href)
        if (normalize(query) === "") {
            url.searchParams.delete("q")
            show([])
            status.textContent = ""
        } else {
            url.searchParams.set("q", query)
            search(query)
        }
        // Coming back from a result finds the same search again.
        window.history.replaceState(null, "", url)
    })
    const given = new URLSearchParams(window.location.search).get("q")
    if (given != null && normalize(given) !== "") {
        input.value = given
        search(given)
    }
})()
