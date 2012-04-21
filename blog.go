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
	view, err = template.New("view").ParseGlob("views/*.html")
	if err != nil {
		panic(err)
	}
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

