package main

import (
	"flag"
	"fmt"
	"html/template"
	"log"
	"net/http"
	"os"
	"sync"

	"code.google.com/p/gorilla/mux"

	auth "github.com/abbot/go-http-auth"
)

var (
	err        error
	view       *template.Template
	viewLocker = &sync.RWMutex{}

	viewFuncs = template.FuncMap{
		"postFormatTime": postFormatTime,
	}

	htpasswd = flag.String("htpasswd",
		"/home/andrew/www/burntsushi.net/.htpasswd",
		"file path to '.htpasswd' file")
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
	checkExists := func(p string) {
		_, err = os.Open(p)
		if err != nil {
			log.Fatalf("%s", err)
		}
	}
	checkExists(*htpasswd)
	checkExists(*postPath)
	checkExists(*viewPath)
	checkExists(*staticPath)

	refreshViews()
	refreshPosts()
}

func main() {
	r := mux.NewRouter()
	r.HandleFunc("/", showIndex)
	r.HandleFunc("/about", showAbout)
	r.HandleFunc("/archives", showArchives)
	r.PathPrefix("/static").
		Handler(http.StripPrefix("/static",
		http.FileServer(http.Dir(*staticPath))))

	// Password protect data refreshing
	authenticator := auth.BasicAuthenticator(
		"Refresh data", auth.HtpasswdFileProvider(*htpasswd))
	r.HandleFunc("/refresh", authenticator(showRefresh))

	// This must be last. Catches and handles anything else as a blog entry.
	r.HandleFunc("/{postname}", showPost)

	http.Handle("/", r)
	http.ListenAndServe(":8081", nil)
}

func refreshViews() {
	viewLocker.Lock()
	defer viewLocker.Unlock()

	// re-parse all the templates
	view, err = template.New("view").Funcs(viewFuncs).
		ParseGlob(*viewPath + "/*.html")
	if err != nil {
		panic(err)
	}
}

func render(w http.ResponseWriter, template string, data interface{}) {
	viewLocker.RLock()
	defer viewLocker.RUnlock()

	err := view.ExecuteTemplate(w, template, data)
	if err != nil {
		panic(err)
	}
}

func render404(w http.ResponseWriter, location string) {
	render(w, "404",
		struct {
			Title    string
			Location string
		}{
			Title:    "Page not found",
			Location: location,
		})
}

func showRefresh(w http.ResponseWriter, req *auth.AuthenticatedRequest) {
	refreshViews()
	refreshPosts()
	fmt.Fprintln(w, "Data refreshed!")
}

func showPost(w http.ResponseWriter, req *http.Request) {
	postsLocker.RLock()
	defer postsLocker.RUnlock()

	vars := mux.Vars(req)
	post := findPost(vars["postname"])
	if post == nil {
		render404(w, fmt.Sprintf(
			"Blog post with identifier '%s'.", vars["postname"]))
		return
	}

	render(w, "post", post)
}

func showIndex(w http.ResponseWriter, req *http.Request) {
	postsLocker.RLock()
	defer postsLocker.RUnlock()

	render(w, "index",
		struct {
			Title string
			Posts Posts
		}{
			Title: "",
			Posts: posts,
		})
}

func showAbout(w http.ResponseWriter, req *http.Request) {
	render(w, "about",
		struct {
			Title string
		}{
			Title: "About",
		})
}

func showArchives(w http.ResponseWriter, req *http.Request) {
	postsLocker.RLock()
	defer postsLocker.RUnlock()

	render(w, "archive",
		struct {
			Title string
			Posts Posts
		}{
			Title: "",
			Posts: posts,
		})
}
