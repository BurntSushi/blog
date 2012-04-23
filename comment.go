package main

import (
	"bytes"
	"fmt"
	"html"
	"html/template"
	"io/ioutil"
	"os"
	"os/exec"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/russross/blackfriday"
)

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

func (c *Comment) String() string {
	return fmt.Sprintf("%s (%s)", c.Name, formatTime(c.Created))
}

// addComment takes an author, email (possibly empty) and a comment and writes
// it to disk. It also checks for malformed input and errors out if anything
// is goofy.
func (p *Post) addComment(author, email, comment string) error {
	author = strings.TrimSpace(author)
	email = strings.TrimSpace(email)
	comment = strings.TrimSpace(comment)

	if len(author) == 0 {
		return fmt.Errorf("Please provide a name.")
	}
	if len(comment) == 0 {
		return fmt.Errorf("Please submit a comment.")
	}
	if strings.Index(author, "\n") != -1 {
		return fmt.Errorf("Please do not put new lines in your name.")
	}
	if strings.Index(email, "\n") != -1 {
		return fmt.Errorf("Please do not put new lines in your email.")
	}

	// Data is valid as far as we know.
	// Use the number of comments (+1) as a unique file name.
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
	for _, notifyEmail := range []string{"jamslam@gmail.com"} {

		buf := bytes.NewBuffer([]byte{})
		err = eview.ExecuteTemplate(buf, "comment-email",
			struct {
				To      string
				Post    *Post
				Comment *Comment
			}{
				To:   notifyEmail,
				Post: p,
				Comment: &Comment{
					Name:     author,
					Email:    email,
					Created:  created,
					Markdown: template.HTML(comment),
				},
			})
		if err != nil {
			logger.Printf("Problem executing email template: %s", err)
			continue
		}

		mailer := exec.Command("mailx", "-t")
		mailer.Stdin = buf
		bts, err := mailer.Start()
	}

	logger.Printf("Added new comment by '%s' for post '%s'.", author, p)

	// Finally done.
	return nil
}

// loadComments reads the comments for a particular post from disk and caches
// them. 
func (p *Post) loadComments() {
	comments := make(Comments, 0)

	files, err := ioutil.ReadDir(commentPath + "/" + p.Ident)
	if err != nil {
		if os.IsNotExist(err) {
			dirName := commentPath + "/" + p.Ident

			err = os.Mkdir(dirName, os.ModeDir | 0770)
			if err != nil {
				logger.Printf("Could not create directory '%s': %s",
					dirName, err)
				return
			}

			// I'm not exactly sure why the above Mkdir's permissions don't
			// stick. It might be because of my umask. That's inconvenient.
			// err = os.Chmod(dirName, os.ModeDir | 0770) 
			// if err != nil { 
				// logger.Printf("Could not set permissions on '%s': %s", 
					// dirName, err) 
				// return 
			// } 

			files, err = ioutil.ReadDir(commentPath + "/" + p.Ident)
		}
		if err != nil {
			logger.Printf("Could not access comment directory for post: %s", p)
			return
		}
	}

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

	p.commentsLocker.Lock()
	p.comments = comments
	p.commentsLocker.Unlock()
}
