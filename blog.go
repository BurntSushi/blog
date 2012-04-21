package main

import (
	"fmt"
	"html/template"
	"net/http"

	"code.google.com/p/gorilla/mux"
)

var (
	err error
	view *template.Template
)

func init() {
	refreshData()
}

func main() {
	r := mux.NewRouter()
	r.HandleFunc("/refresh", showRefresh) // dev only!
	r.HandleFunc("/", showIndex)
	r.PathPrefix("/static").
		Handler(http.StripPrefix("/static",
								 http.FileServer(http.Dir("./static"))))

	http.Handle("/", r)
	http.ListenAndServe(":8081", nil)
}

func refreshData() {
	view, err = template.New("view").ParseGlob("views/*.html")
	if err != nil {
		panic(err)
	}
}

func showRefresh(w http.ResponseWriter, req *http.Request) {
	refreshData()
	fmt.Fprintln(w, "Data refreshed!")
}

func showIndex(w http.ResponseWriter, req *http.Request) {
	err := view.ExecuteTemplate(w, "index",
		struct {
			Title string
		}{
			Title: "Hiya :-)",
		})
	if err != nil {
		panic(err)
	}
}

