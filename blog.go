package main

import (
	"html/template"
	"net/http"

	"code.google.com/p/gorilla/mux"
)

var (
	view *template.Template
)

func init() {
	view = template.New("view").ParseGlob("views/*.html")
}

func main() {
	r := mux.NewRouter()
	r.HandleFunc("/", showIndex)

	http.Handle("/", r)
	http.ListenAndServe(":8081", nil)
}

func mainIndex(w http.ResponseWriter, req *http.Request) {
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

