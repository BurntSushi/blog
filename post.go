package main

import (
	"fmt"
	"html/template"
	"io/ioutil"
	"sort"
	"strings"
	"sync"
	"time"

	"github.com/russross/blackfriday"
)

var (
	posts       Posts
	postsLocker = &sync.RWMutex{}
)

func PostsGet() Posts {
	postsLocker.RLock()
	defer postsLocker.RUnlock()

	return posts
}

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

type Post struct {
	Title    string
	Ident    string
	Created  time.Time
	Raw      string
	Markdown template.HTML

	comments       Comments
	commentsLocker *sync.RWMutex
	addCommentLocker *sync.Mutex
}

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
	p.addCommentLocker = &sync.Mutex{}

	// Now load the comments!
	// (This always succeeds. If there are any errors with individual comments,
	//  they are simply omitted from the resulting list.)
	p.loadComments()

	return p, nil
}

// CommentsGet returns a copy of the comments for a Post.
func (p *Post) CommentsGet() Comments {
	p.commentsLocker.RLock()
	defer p.commentsLocker.RUnlock()

	return p.comments
}

func (p *Post) String() string {
	return fmt.Sprintf("%s [%s] (%s)", p.Title, p.Ident,
		formatTime(p.Created))
}

func findPost(ident string) *Post {
	for _, post := range PostsGet() {
		if strings.ToLower(ident) == strings.ToLower(post.Ident) {
			return post
		}
	}
	return nil
}

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
