package main

/*
	comment.go contains methods and functions related to the processing of
	comments. 

	Such processing includes adding comments, sanitizing user input,
	validating user input, reading comments from disk and caching comments
	in memory.

	Most of the methods here are defined with a *Post receiver, but they
	are specifically related to processing comments of a particular *Post.
*/

import (
	"fmt"
	"html"
	"html/template"
	"io/ioutil"
	"os"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/russross/blackfriday"
)

// Comments exists to implement sort.Interface
type Comments []*Comment

func (cs Comments) Len() int {
	return len(cs)
}

func (cs Comments) Less(i, j int) bool {
	return cs[i].Created.After(cs[j].Created)
}

func (cs Comments) Swap(i, j int) {
	cs[i], cs[j] = cs[j], cs[i]
}

// Comment stores all information related to a comment.
// 'Markdown' is the HTML version of the comment.
type Comment struct {
	Name     string
	Email    string
	Created  time.Time
	Markdown template.HTML
}

// newComment reads a comment file from disk and stores it in a Comment struct.
// The first three lines of a comment file correspond to the name of the
// author, the email address of the author and the date posted. (The email
// address may be empty, but there will still be a new line.)
// The rest of the lines correspond to the comment body.
func newComment(postIdent, fileName string) (*Comment, error) {
	c := new(Comment)

	raw, err := ioutil.ReadFile(commentPath + "/" + postIdent + "/" + fileName)
	if err != nil {
		return nil, err
	}
	lines := strings.Split(string(raw), "\n")

	c.Name = strings.TrimSpace(lines[0])
	c.Email = strings.TrimSpace(lines[1])

	nanosecs, err := strconv.ParseInt(strings.TrimSpace(lines[2]), 0, 64)
	if err != nil {
		return nil, err
	}
	c.Created = timeFromNano(nanosecs)

	// Now parse the rest of the lines as markdown
	body := strings.Join(lines[3:], "\n")
	c.Markdown = template.HTML(blackfriday.MarkdownCommon([]byte(body)))

	return c, nil
}

// String is a string representation of a comment: its author and timestamp.
func (c *Comment) String() string {
	return fmt.Sprintf("%s (%s)", c.Name, formatTime(c.Created))
}

// addComment takes an author, email (possibly empty) and a comment and writes
// it to disk. It also checks for malformed input and errors if anything
// is goofy.
func (p *Post) addComment(author, email, comment string) error {
	author = strings.TrimSpace(author)
	email = strings.TrimSpace(email)
	comment = strings.TrimSpace(comment)

	if err := validateComment(author, email, comment); err != nil {
		return err
	}

	// Data is valid as far as we know.
	// Use the number of comments (+1) as a unique file name.
	// We can do this because we're inside an addCommentLocker for this entry.
	fileName := fmt.Sprintf("%s/%s/%d", commentPath, p.Ident,
		len(p.CommentsGet())+1)

	// Build the lines of the file.
	created := time.Now()
	unixNano := fmt.Sprintf("%d", created.UnixNano())
	lines := []string{author, email, unixNano, comment}

	// Sanitize!
	for i, userText := range lines {
		lines[i] = html.EscapeString(userText)
	}

	// Now write the file.
	err := ioutil.WriteFile(fileName, []byte(strings.Join(lines, "\n")), 0660)
	if err != nil {
		logger.Printf("There was an error adding a comment: %s", err)
		return fmt.Errorf("An unknown error occurred when trying to " +
			"submit your comment. Please contact admin@burntsushi.net to " +
			"report a bug in saving new comments.")
	}

	// Do some email notifications!
	p.notify(&Comment{
		Name:     author,
		Email:    email,
		Created:  created,
		Markdown: template.HTML(comment),
	})

	logger.Printf("Added new comment by '%s' for post '%s'.", author, p)
	return nil
}

// loadComments reads the comments for a particular post from disk and caches
// them. They are also sorted.
// 'loadCommentsLocker' is needed to make sure two instances of 'loadComments'
// don't run in parallel.
func (p *Post) loadComments() {
	p.loadCommentsLocker.Lock()
	defer p.loadCommentsLocker.Unlock()

	files := p.commentFiles()
	if files == nil {
		return
	}

	comments := make(Comments, 0)
	for _, file := range files {
		if c, err := newComment(p.Ident, file.Name()); err == nil {
			logger.Printf("\tLoaded comment '%s' successfully.", c)
			comments = append(comments, c)
		} else {
			logger.Printf("\t%s", err)
			logger.Printf("\tCould not load comment '%s'. Skipping...",
				file.Name())
		}
	}
	sort.Sort(comments)

	p.CommentsSet(comments)
}

// commentFiles returns a slice of comment files from disk for a particular
// entry. If an entry's comment directory doesn't exist, it is created.
func (p *Post) commentFiles() []os.FileInfo {
	files, err := ioutil.ReadDir(commentPath + "/" + p.Ident)
	if err != nil {
		if os.IsNotExist(err) {
			dirName := commentPath + "/" + p.Ident

			err = os.Mkdir(dirName, os.ModeDir|0770)
			if err != nil {
				logger.Printf("Could not create directory '%s': %s",
					dirName, err)
				return nil
			}

			files, err = ioutil.ReadDir(commentPath + "/" + p.Ident)
		}
		if err != nil {
			logger.Printf("Could not access comment directory for post: %s", p)
			return nil
		}
	}
	return files
}

// validateComment takes all user input associated with a comment and runs
// a number of tests on it to make sure it's OK to accept.
// The return value will be nil if the comment is valid.
func validateComment(author, email, comment string) error {
	if len(author) == 0 {
		return fmt.Errorf("Please provide a name.")
	}
	if len(author) > 256 {
		return fmt.Errorf(
			"Please shorten your name to less than 256 characters.")
	}
	if len(email) > 256 {
		return fmt.Errorf(
			"Please shorten your email address to less than 256 characters.")
	}
	if len(comment) == 0 {
		return fmt.Errorf("Please submit a comment.")
	}
	if len(comment) > 200000 {
		return fmt.Errorf(
			"There is a 200,000 character limit on comments. If you really " +
				"need to post something longer, please separate your post " +
				"into multiple comments. Sorry for the inconvenience.")
	}
	if strings.Index(author, "\n") != -1 {
		return fmt.Errorf("Please do not put new lines in your name.")
	}
	if strings.Index(email, "\n") != -1 {
		return fmt.Errorf("Please do not put new lines in your email.")
	}

	return nil
}
