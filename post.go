package main

import (
	"bytes"
	"fmt"
	"html/template"
	"io/ioutil"
	"os/exec"
	"sort"
	"strings"
	"sync"
	"time"

	"github.com/russross/blackfriday"
)

var (
	posts       Posts             // global var containing cached posts
	postsLocker = &sync.RWMutex{} // make posts thread-safe
)

// PostsGet centralizes reading the posts pointer. The 'posts' global
// variable should never be accessed directly.
func PostsGet() Posts {
	postsLocker.RLock()
	defer postsLocker.RUnlock()

	return posts
}

// Posts implements sort.Interface to sort posts in descending order by time.
type Posts []*Post

func (ps Posts) Len() int {
	return len(ps)
}

func (ps Posts) Less(i, j int) bool {
	return ps[i].Created.After(ps[j].Created)
}

func (ps Posts) Swap(i, j int) {
	ps[i], ps[j] = ps[j], ps[i]
}

// Post contains all information relevant to blog posts.
// It includes a couple of mutexes to keep comment processing thread safe.
// Raw is not currently used but probably will in the future if blog
// posts are to be displayed in different formats (i.e., RSS).
type Post struct {
	Title    string        // title of blog post
	Ident    string        // URL unique identifier of blog post
	Created  time.Time     // time created
	Raw      string        // raw content with markdown markup
	Markdown template.HTML // markdown converted to HTML

	comments           Comments      // sorted list of comments for this entry
	commentsLocker     *sync.RWMutex // reads/writes of comments are thread safe
	loadCommentsLocker *sync.Mutex   // only one loadComments at a time
	addCommentLocker   *sync.Mutex   // enforces one new comment at a time
}

// newPost reads a blog post (in markdown format) from disk and caches it
// to memory. newPost is smart and inspects the contents of the blog post
// markdown file for special information. Particularly, the file name of
// the blog post is in the format YYYY-MM-DD-HH-MM.md and the first two
// lines of the post contain the unique URL identifier (Ident) and title (Title)
// of the post, respectively.
func newPost(fileName string) (*Post, error) {
	p := new(Post)

	// Load the raw markdown content.
	raw, err := ioutil.ReadFile(postPath + "/" + fileName)
	if err != nil {
		return nil, err
	}
	p.Raw = string(raw)

	// Find the "ident" in the raw content.
	// This is used in the URL.
	// It's enclosed in <!-- and --> in the first line.
	firstNL := strings.Index(p.Raw, "\n")
	if firstNL == -1 {
		return nil, fmt.Errorf("Could not find Ident in '%s'.", fileName)
	} else {
		p.Ident = strings.Trim(p.Raw[:firstNL], "<!--> ")
	}

	// Now find the title in the raw content. Trim '#' and whitespace.
	if secondNL := strings.Index(p.Raw[firstNL+1:], "\n"); secondNL == -1 {
		logger.Printf("Could not find title in blog post: %s", fileName)
		p.Title = "N/A"
	} else {
		secondNL += firstNL + 1
		p.Title = strings.Trim(p.Raw[firstNL+1:secondNL], "# ")
		p.Raw = p.Raw[secondNL+1:]
	}

	// Parse raw post as markdown
	// Note that we move the ident AND the title from the raw post.
	p.Markdown = template.HTML(blackfriday.MarkdownCommon([]byte(p.Raw)))

	// Parse the date
	created, err := time.Parse("2006-01-02-15-04", fileName[:len(fileName)-3])
	if err != nil {
		return nil, err
	}
	p.Created = created

	// Setup the comments lockers
	p.commentsLocker = &sync.RWMutex{}
	p.loadCommentsLocker = &sync.Mutex{}
	p.addCommentLocker = &sync.Mutex{}

	// Now load the comments!
	// (This always succeeds. If there are any errors with individual comments,
	//  they are simply omitted from the resulting list.)
	p.loadComments()

	return p, nil
}

// CommentsGet returns a pointer to the comments for a Post.
// Post.comments should never be used directly, else thread safety is lost.
func (p *Post) CommentsGet() Comments {
	p.commentsLocker.RLock()
	defer p.commentsLocker.RUnlock()

	return p.comments
}

// notify takes a new comment and sends an email to all applicable parties.
// I'm aware of net/smtp, but I have two problems:
// 1) I couldn't figure out how to get it to work with GMail's smtp servers.
// 2) The fewer places I store my GMail login, the better. mailx is fine
//    for such simple purposes.
func (p *Post) notify(comment *Comment) {
	viewLocker.RLock()
	defer viewLocker.RUnlock()

	var err error

	for _, notifyEmail := range []string{"jamslam@gmail.com"} {
		buf := bytes.NewBuffer([]byte{})
		err = tview.ExecuteTemplate(buf, "comment-email",
			struct {
				To      string
				Post    *Post
				Comment *Comment
			}{
				To:      notifyEmail,
				Post:    p,
				Comment: comment,
			})
		if err != nil {
			logger.Printf("Problem executing email template: %s", err)
			continue
		}

		mailer := exec.Command("mailx", "-t")
		mailer.Stdin = buf
		err := mailer.Start()
		if err != nil {
			logger.Printf("Could not run 'mailx' because: %s", err)
		}
	}
}

// String is a string representation of a post. Namely, its title, unique
// URL identifier and its timestamp.
func (p *Post) String() string {
	return fmt.Sprintf("%s [%s] (%s)", p.Title, p.Ident,
		formatTime(p.Created))
}

// findPost looks for a post with Ident matching ident in the cache.
// If none are found, nil is returned.
func findPost(ident string) *Post {
	for _, post := range PostsGet() {
		if strings.ToLower(ident) == strings.ToLower(post.Ident) {
			return post
		}
	}
	return nil
}

// refreshPosts reloads the cache of posts from disk. It is only run
// on startup and explicitly whenever '/refresh' is visited. (Don't even think
// about it :P It requires a username and password. :P)
func refreshPosts() {
	files, err := ioutil.ReadDir(postPath)
	if err != nil {
		logger.Fatal(err)
	}

	newPosts := make(Posts, 0, len(files))
	for _, file := range files {
		if !strings.HasSuffix(file.Name(), ".md") {
			continue
		}

		logger.Printf("Trying to update '%s'...", file.Name())
		if p, err := newPost(file.Name()); err == nil {
			logger.Printf("Updated '%s' successfully.", p)
			newPosts = append(newPosts, p)
		} else {
			logger.Println(err)
			logger.Printf("Could not update '%s'. Skipping...", file.Name())
		}
	}
	sort.Sort(newPosts)

	postsLocker.Lock()
	posts = newPosts
	postsLocker.Unlock()
}
