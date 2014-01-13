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
	// current working directory; a context for relative paths defined below
	cwd = flag.String("cwd",
		"/home/andrew/www/burntsushi.net/blog",
		"path to blog directory that contains "+
			"'posts', 'views', 'static' and 'log'")

	// file for .htpasswd that protects /refresh. Keep it out of the repo!
	htpasswd = flag.String("htpasswd",
		"/home/andrew/www/burntsushi.net/.htpasswd",
		"file path to '.htpasswd' file")

	view   *template.Template    // html view templates
	tview  *txtTemplate.Template // text view templates
	logger *log.Logger           // custom logger for my blog

	postPath    string // relative directory containing blog posts
	commentPath string // relative directory containing comment directories
	viewPath    string // relative directory contain HTML and TEXT templates
	staticPath  string // relative directory containing all static files
	logPath     string // relative directory containing any log output

	viewLocker = &sync.RWMutex{} // keep views thread-safe when we update them

	tviewFuncs = txtTemplate.FuncMap{
		"formatTime": formatTime,
		"pluralize":  pluralize,
	}
	viewFuncs = template.FuncMap{
		"formatTime": formatTime,
		"pluralize":  pluralize,
	}
)

// init checks the paths specified to make sure they are readable.
// It also initializes the logger as soon as we can.
// And finally caches the views, posts and comments currently on disk.
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

// main sets up the routes (thanks Gorilla!).
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

// render is a single-point-of-truth for executing HTML templates.
// It's particularly useful in that it encapsulates the viewLocker.
// (i.e., we make sure we aren't reading a view while it's being updated.)
func render(w http.ResponseWriter, template string, data interface{}) {
	viewLocker.RLock()
	defer viewLocker.RUnlock()

	err := view.ExecuteTemplate(w, template, data)
	if err != nil {
		panic(err)
	}
}

// renderPost builds on 'render' but is specifically for showing a post's page.
// Namely, it handles the population of the "add comment" form.
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

// render404 builds on 'render' but forces a special '404' template when
// a particular page cannot be found.
// Currently, this is only used when an invalid post identifier is used in
// the URL.
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

// showIndex renders the index page.
// It should also limit the number of posts displayed, but I don't have
// that many yet.
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

// showAbout renders the about page.
func showAbout(w http.ResponseWriter, req *http.Request) {
	render(w, "about",
		struct {
			Title string
		}{
			Title: "About",
		})
}

// showArchives renders the blog archive page.
// The archives page is currently a list of all blog posts and their timestamps.
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

// showPost finds a post with the corresponding ident matching "postname"
// and displays it. If one cannot be found, a 404 error is shown.
func showPost(w http.ResponseWriter, req *http.Request) {
	vars := mux.Vars(req)
	post := forceValidPost(w, vars["postname"])
	if post == nil {
		return
	}

	renderPost(w, post, "", "", "", "")
}

// addComment responds to a POST request for adding a comment to a post
// with an ident matching "postname". If such a post cannot be found,
// a 404 page is shown.
// Form data is trimmed and the CAPTCHA is verified before anything else.
// Finally, we add the comment but make sure to wrap it in an addCommentLocker.
// This makes it so only a single comment can be added at a time for one
// particular entry. This allows us to rely on the existing cache to provide
// a unique identifier as a comment file name. (i.e., an incrementing integer.)
func addComment(w http.ResponseWriter, req *http.Request) {
	render404(w, "add comment")
	return

	vars := mux.Vars(req)
	post := forceValidPost(w, vars["postname"])
	if post == nil {
		return
	}

	// Get the form values.
	author := strings.TrimSpace(req.FormValue("plato"))
	email := strings.TrimSpace(req.FormValue("email"))
	comment := strings.TrimSpace(req.FormValue("cauchy"))

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

// showRefresh completely reloads the view, post and comment caches from disk.
// It prints a silly message in response.
// Note that this page is password protected. (Thanks abbot!)
func showRefresh(w http.ResponseWriter, req *auth.AuthenticatedRequest) {
	refreshViews()
	refreshPosts()
	fmt.Fprintln(w, "Data refreshed!")
}

// refreshViews reparses the template files in the views directory.
// This is useful when we update some HTML and don't want to restart the server.
// Aside from startup, these are only refreshed when '/refresh' is visited.
func refreshViews() {
	viewLocker.Lock()
	defer viewLocker.Unlock()

	var err error

	// re-parse all the templates
	view, err = template.New("view").Funcs(viewFuncs).
		ParseGlob(viewPath + "/*.html")
	if err != nil {
		panic(err)
	}

	tview, err = txtTemplate.New("view").Funcs(tviewFuncs).
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
