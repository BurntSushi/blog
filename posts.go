package main

import (
	"fmt"
	"html/template"
	"io/ioutil"
	"log"
	"sort"
	"strings"
	"time"

	"github.com/russross/blackfriday"
)

var (
	posts Posts
)

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
	Title string
	Ident string
	Created time.Time
	Raw string
	Markdown template.HTML
}

func (p *Post) String() string {
	return fmt.Sprintf("%s [%s] (%s)", p.Title, p.Ident,
		p.Created.Format("01/02/2006 at 3:04pm"))
}

func postFormatTime(t time.Time) string {
	return t.Format("01/02/2006 at 3:04pm")
}

func findPost(ident string) *Post {
	for _, post := range posts {
		if strings.ToLower(ident) == strings.ToLower(post.Ident) {
			return post
		}
	}
	return nil
}

func refreshPosts() {
	files, err := ioutil.ReadDir(*postPath)
	if err != nil {
		log.Fatal(err)
	}

	posts = make(Posts, 0, len(files))
	for _, file := range files {
		if !strings.HasSuffix(file.Name(), ".md") {
			continue
		}
		p := new(Post)
		name := file.Name()

		// Load the raw markdown content.
		raw, err := ioutil.ReadFile(*postPath + "/" + name)
		p.Raw = string(raw)
		if err != nil {
			log.Println(err)
			log.Println("Skipping '%s'...", name)
		}

		// Find the "ident" in the raw content.
		// This is used in the URL.
		// It's enclosed in <!-- and --> in the first line.
		firstNL := strings.Index(p.Raw, "\n")
		if firstNL == -1 {
			log.Printf("Could not find Ident in '%s'. Skipping.", name)
			continue
		} else {
			p.Ident = strings.Trim(p.Raw[:firstNL], "<!--> ")
		}

		// Now find the title in the raw content. Trim '#' and whitespace.
		if secondNL := strings.Index(p.Raw[firstNL+1:], "\n"); secondNL > -1 {
			secondNL += firstNL + 1
			p.Title = strings.Trim(p.Raw[firstNL+1:secondNL], "# ")
			p.Raw = p.Raw[secondNL+1:]
		} else {
			log.Printf("Could not find title in blog post: %s", name)
			p.Title = "N/A"
		}

		// Parse raw post as markdown
		// Note that we move the ident AND the title from the raw post.
		p.Markdown = template.HTML(blackfriday.MarkdownCommon([]byte(p.Raw)))

		// Parse the date
		created, err := time.Parse("2006-01-02-15-04", name[:len(name) - 3])
		if err != nil {
			log.Println(err)
			log.Println("Skipping '%s'...", name)
			continue
		}
		p.Created = created

		posts = append(posts, p)
	}

	sort.Sort(posts)

	for _, p := range posts {
		fmt.Println(p)
	}
}

