package main

import (
	"flag"
	"fmt"
	"html/template"
	"log"
	"net/http"
	"os"
	"strings"
	"sync"
	"syscall"
	txtTemplate "text/template"

	"code.google.com/p/gorilla/mux"

	auth "github.com/abbot/go-http-auth"

	"github.com/dchest/captcha"
)

var (
	err                                                  error
	view                                                 *template.Template
	eview                                                *txtTemplate.Template
	logger                                               *log.Logger
	postPath, commentPath, viewPath, staticPath, logPath string

	viewLocker = &sync.RWMutex{}

	eviewFuncs = txtTemplate.FuncMap{
		"formatTime": formatTime,
		"pluralize":  pluralize,
	}
	viewFuncs = template.FuncMap{
		"formatTime": formatTime,
		"pluralize":  pluralize,
	}

	htpasswd = flag.String("htpasswd",
		"/home/andrew/www/burntsushi.net/.htpasswd",
		"file path to '.htpasswd' file")
	cwd = flag.String("cwd",
		"/home/andrew/www/burntsushi.net/blog",
		"path to blog directory that contains "+
			"'posts', 'views', 'static' and 'log'")
)

func init() {
	// let my people go!
	syscall.Umask(0)

	flag.Parse()

	postPath = *cwd + "/posts"
	commentPath = *cwd + "/comments"
	viewPath = *cwd + "/views"
	staticPath = *cwd + "/static"
	logPath = *cwd + "/log"

	// Create the logger first.
	logFile, err := os.OpenFile(logPath+"/blog.log",
		os.O_WRONLY|os.O_APPEND|os.O_CREATE, 0666)
	if err != nil {
		panic(err)
	}
	logger = log.New(logFile, "BLOG LOG: ", log.Ldate|log.Ltime)

	logger.Println("----------------------------------------------------------")
	logger.Println("Starting BLOG server...")

	// Make sure our necessary directories and files are readable.
	checkExists := func(p string) {
		_, err = os.Open(p)
		if err != nil {
			logger.Fatalf("%s\n", err)
		}
	}
	checkExists(*cwd)
	checkExists(postPath)
	checkExists(commentPath)
	checkExists(viewPath)
	checkExists(staticPath)
	checkExists(logPath)
	checkExists(*htpasswd)

	// Initialize views, posts and comments.
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
		http.FileServer(http.Dir(staticPath))))

	// Password protect data refreshing
	authenticator := auth.BasicAuthenticator(
		"Refresh data", auth.HtpasswdFileProvider(*htpasswd))
	r.HandleFunc("/refresh", authenticator(showRefresh))

	// These must be last. The first shows blog posts, the second adds comments.
	r.HandleFunc("/{postname}", showPost).Methods("GET")
	r.HandleFunc("/{postname}", addComment).Methods("POST")

	// Captcha!
	http.Handle("/captcha/",
		captcha.Server(captcha.StdWidth, captcha.StdHeight))

	// Okay, let Gorilla do its work.
	http.Handle("/", r)
	http.ListenAndServe(":8081", nil)
}

func refreshViews() {
	viewLocker.Lock()
	defer viewLocker.Unlock()

	// re-parse all the templates
	view, err = template.New("view").Funcs(viewFuncs).
		ParseGlob(viewPath + "/*.html")
	if err != nil {
		panic(err)
	}

	eview, err = txtTemplate.New("view").Funcs(eviewFuncs).
		ParseGlob(viewPath + "/*.txt")
	if err != nil {
		panic(err)
	}
}

// forceValidPost takes a post identifier and finds the corresponding
// post and returns it. If one cannot be found, a 404 page is rendered
// and nil is returned.
func forceValidPost(w http.ResponseWriter, postIdent string) *Post {
	post := findPost(postIdent)
	if post == nil {
		logger.Printf("Could not find post with identifier '%s'.", postIdent)
		render404(w, fmt.Sprintf(
			"Blog post with identifier '%s'.", postIdent))
		return nil
	}
	return post
}

func render(w http.ResponseWriter, template string, data interface{}) {
	viewLocker.RLock()
	defer viewLocker.RUnlock()

	err := view.ExecuteTemplate(w, template, data)
	if err != nil {
		panic(err)
	}
}

func renderPost(w http.ResponseWriter, post *Post,
	formError, formAuthor, formEmail, formComment string) {

	render(w, "post",
		struct {
			*Post
			FormCaptchaId string
			FormError     string
			FormAuthor    string
			FormEmail     string
			FormComment   string
		}{
			Post:          post,
			FormCaptchaId: captcha.New(),
			FormError:     formError,
			FormAuthor:    formAuthor,
			FormEmail:     formEmail,
			FormComment:   formComment,
		})
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
	vars := mux.Vars(req)
	post := forceValidPost(w, vars["postname"])
	if post == nil {
		return
	}

	renderPost(w, post, "", "", "", "")
}

func addComment(w http.ResponseWriter, req *http.Request) {
	vars := mux.Vars(req)
	post := forceValidPost(w, vars["postname"])
	if post == nil {
		return
	}

	// Get the form values.
	author := strings.TrimSpace(req.FormValue("author"))
	email := strings.TrimSpace(req.FormValue("email"))
	comment := strings.TrimSpace(req.FormValue("comment"))

	// First check the captcha before anything else.
	captchaId := req.FormValue("captchaid")
	userTest := req.FormValue("captcha")
	if !captcha.VerifyString(captchaId, userTest) {
		renderPost(w, post,
			"The CAPTCHA text you entered did not match the text in the "+
				"image. Please try again.", author, email, comment)
		return
	}

	// We need to make sure only one comment per post can be added at a time.
	// Namely, we need unique sequential identifiers for each comment.
	post.addCommentLocker.Lock()
	defer post.addCommentLocker.Unlock()

	// Add the comment and refresh the comment store for this post.
	// 'addComment' makes sure the input is valid and reports an
	// error otherwise.
	err := post.addComment(author, email, comment)
	if err == nil { // success!
		post.loadComments()
		http.Redirect(w, req, "/"+post.Ident+"#comments", http.StatusFound)
	} else { // failure... :-(
		renderPost(w, post, err.Error(), author, email, comment)
	}
}

func showIndex(w http.ResponseWriter, req *http.Request) {
	render(w, "index",
		struct {
			Title string
			Posts Posts
		}{
			Title: "",
			Posts: PostsGet(),
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
	render(w, "archive",
		struct {
			Title string
			Posts Posts
		}{
			Title: "",
			Posts: PostsGet(),
		})
}
