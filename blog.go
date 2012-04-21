package main

import (
	"flag"
	"fmt"
	"html/template"
	"io"
	"log"
	"net/http"
	"os"

	"code.google.com/p/gorilla/mux"
)

var (
	err  error
	view *template.Template

	viewFuncs = template.FuncMap{
		"postFormatTime": postFormatTime,
	}

	postPath = flag.String("post",
		"/home/andrew/www/burntsushi.net/blog/posts",
		"path to 'posts' directory")
	viewPath = flag.String("view",
		"/home/andrew/www/burntsushi.net/blog/views",
		"path to 'view' directory")
	staticPath = flag.String("static",
		"/home/andrew/www/burntsushi.net/blog/static",
		"path to 'static' directory")
)

func init() {
	flag.Parse()

	// Make sure our view/static directories are readable.
	checkDir := func(p string) {
		_, err = os.Open(p)
		if err != nil {
			log.Fatalf("%s", err)
		}
	}
	checkDir(*postPath)
	checkDir(*viewPath)
	checkDir(*staticPath)

	refreshData()
}

func main() {
	r := mux.NewRouter()
	r.HandleFunc("/refresh", showRefresh) // dev only!
	r.HandleFunc("/", showIndex)
	r.HandleFunc("/about", showAbout)
	r.HandleFunc("/archives", showArchives)
	r.PathPrefix("/static").
		Handler(http.StripPrefix("/static",
		http.FileServer(http.Dir(*staticPath))))
	r.HandleFunc("/{postname}", showPost)

	http.Handle("/", r)
	http.ListenAndServe(":8081", nil)
}

func refreshData() {
	// re-parse all the templates
	view, err = template.New("view").Funcs(viewFuncs).
		ParseGlob(*viewPath + "/*.html")
	if err != nil {
		panic(err)
	}

	// now re-load all of the blog entries
	refreshPosts()
}

func render404(w io.Writer, location string) {
	err := view.ExecuteTemplate(w, "404",
		struct {
			Title string
			Location string
		}{
			Title: "Page not found",
			Location: location,
		})
	if err != nil {
		panic(err)
	}
}

func showRefresh(w http.ResponseWriter, req *http.Request) {
	refreshData()
	fmt.Fprintln(w, "Data refreshed!")
}

func showPost(w http.ResponseWriter, req *http.Request) {
	vars := mux.Vars(req)
	post := findPost(vars["postname"])
	if post == nil {
		render404(w, fmt.Sprintf(
			"Blog post with identifier '%s'.", vars["postname"]))
		return
	}

	err := view.ExecuteTemplate(w, "post", post)
	if err != nil {
		panic(err)
	}
}

func showIndex(w http.ResponseWriter, req *http.Request) {
	err := view.ExecuteTemplate(w, "index",
		struct {
			Title string
			Posts Posts
		}{
			Title: "",
			Posts: posts,
		})
	if err != nil {
		panic(err)
	}
}

func showAbout(w http.ResponseWriter, req *http.Request) {
	err := view.ExecuteTemplate(w, "about",
		struct {
			Title string
		}{
			Title: "About",
		})
	if err != nil {
		panic(err)
	}
}

func showArchives(w http.ResponseWriter, req *http.Request) {
	err := view.ExecuteTemplate(w, "archive",
		struct {
			Title string
		}{
			Title: "Blog Archives",
		})
	if err != nil {
		panic(err)
	}
}
